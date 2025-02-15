use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::Result;
use rust_i18n::t;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::{is_dumb, print_separator};
use crate::utils::{require_option, which};
use crate::Step;

pub struct Powershell {
    path: Option<PathBuf>,
    profile: Option<PathBuf>,
}

impl Powershell {
    pub fn new() -> Self {
        let path = which("pwsh").or_else(|| which("powershell")).filter(|_| !is_dumb());
        let profile = Self::get_profile(&path);
        Powershell { path, profile }
    }

    #[cfg(windows)]
    pub fn windows_powershell() -> Self {
        Powershell {
            path: which("powershell").filter(|_| !is_dumb()),
            profile: None,
        }
    }

    // Added getter method for profile.
    pub fn profile(&self) -> Option<&PathBuf> {
        self.profile.as_ref()
    }

    fn get_profile(path: &Option<PathBuf>) -> Option<PathBuf> {
        path.as_ref().and_then(|path| {
            Command::new(path)
                .args(["-NoProfile", "-Command", "Split-Path $profile"])
                .output_checked_utf8()
                .map(|output| PathBuf::from(output.stdout.trim()))
                .and_then(super::super::utils::PathExt::require)
                .ok()
        })
    }

    /// Shared logic to build a command, with support for sudo and common arguments.
    fn build_command_internal<'a>(
        &self,
        ctx: &'a ExecutionContext,
        additional_args: &[&str],
    ) -> Result<impl CommandExt + 'a> {
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;
        let executor = &mut ctx.run_type();
        let mut command = if let Some(sudo) = ctx.sudo() {
            let mut cmd = executor.execute(sudo);
            cmd.arg(powershell);
            cmd
        } else {
            executor.execute(powershell)
        };

        // Windows-specific extensions are applied separately.
        #[cfg(windows)]
        {
            if let Some(policy_args) = self.execution_policy_args_if_needed() {
                command.args(policy_args);
            }
        }

        command.args(Self::common_args()).args(additional_args);
        Ok(command)
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("Powershell Modules Update"));

        let mut cmd_args = vec!["Update-Module"];
        if ctx.config().verbose() {
            cmd_args.push("-Verbose");
        }
        if ctx.config().yes(Step::Powershell) {
            cmd_args.push("-Force");
        }

        println!("{}", t!("Updating modules..."));
        self.build_command_internal(ctx, &cmd_args)?.status_checked()
    }

    fn common_args() -> &'static [&'static str] {
        &["-NoProfile"]
    }

    #[cfg(windows)]
    pub fn supports_windows_update(&self) -> bool {
        self.path
            .as_ref()
            .map(|p| Self::has_module(p, "PSWindowsUpdate"))
            .unwrap_or(false)
    }

    #[cfg(windows)]
    fn execution_policy_args_if_needed(&self) -> Option<&'static [&'static str]> {
        if self.is_execution_policy_set("RemoteSigned") {
            None
        } else {
            Some(&["-ExecutionPolicy", "RemoteSigned", "-Scope", "Process"])
        }
    }

    #[cfg(windows)]
    fn is_execution_policy_set(&self, policy: &str) -> bool {
        if let Some(powershell) = &self.path {
            let output = Command::new(powershell)
                .args(["-NoProfile", "-Command", "Get-ExecutionPolicy -Scope Process"])
                .output_checked_utf8();

            if let Ok(output) = output {
                return output.stdout.trim() == policy;
            }
        }
        false
    }

    #[cfg(windows)]
    pub fn windows_update(&self, ctx: &ExecutionContext) -> Result<()> {
        debug_assert!(self.supports_windows_update());

        // Windows-specific command to update Windows
        self.build_command_internal(ctx, &["Install-WindowsUpdate -Verbose"])
            .map(|mut cmd| {
                if ctx.config().accept_all_windows_updates() {
                    cmd.arg("-AcceptAll");
                }
                cmd
            })?
            .status_checked()
    }

    #[cfg(windows)]
    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        println!("{}", t!("Scanning for updates..."));

        let update_command = "(Get-CimInstance -Namespace \"Root\\cimv2\\mdm\\dmmap\" \
                                 -ClassName \"MDM_EnterpriseModernAppManagement_AppManagement01\" | \
                                 Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue";
        self.build_command_internal(ctx, &["-Command", update_command])?
            .output_checked_with_utf8(|output| {
                if output.stdout.trim() == "0" {
                    println!(
                        "{}",
                        t!("Success, Microsoft Store apps are being updated in the background")
                    );
                    Ok(())
                } else {
                    println!(
                        "{}",
                        t!("Unable to update Microsoft Store apps, manual intervention is required")
                    );
                    Err(())
                }
            })
            .map(|_| ())
    }

    #[cfg(windows)]
    fn has_module(powershell: &PathBuf, command: &str) -> bool {
        Command::new(powershell)
            .args(&[
                "-NoProfile",
                "-Command",
                &format!("Get-Module -ListAvailable {}", command),
            ])
            .output_checked_utf8()
            .map(|result| !result.stdout.is_empty())
            .unwrap_or(false)
    }
}
