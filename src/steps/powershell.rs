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
        let profile = path.as_ref().and_then(Self::get_profile);
        Powershell { path, profile }
    }

    #[cfg(windows)]
    pub fn windows_powershell() -> Self {
        Powershell {
            path: which("powershell").filter(|_| !is_dumb()),
            profile: None,
        }
    }

    pub fn profile(&self) -> Option<&PathBuf> {
        self.profile.as_ref()
    }

    fn get_profile(path: &PathBuf) -> Option<PathBuf> {
        Self::execute_with_command(path, &["-NoProfile", "-Command", "Split-Path $PROFILE"], |stdout| {
            Ok(stdout)
        })
        .ok() // Convert the Result<String> to Option<String>
        .and_then(|s| super::super::utils::PathExt::require(PathBuf::from(s)).ok())
    }

    fn execute_with_command<F>(path: &PathBuf, args: &[&str], f: F) -> Result<String>
    where
        F: FnOnce(String) -> Result<String>,
    {
        let output = Command::new(path).args(args).output_checked_utf8()?;
        let stdout = output.stdout.trim().to_string();
        f(stdout)
    }

    /// Builds a command with common arguments and optional sudo support.
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
    pub fn execution_policy_args_if_needed(&self) -> Option<&'static [&'static str]> {
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
}

#[cfg(windows)]
impl Powershell {
    pub fn supports_windows_update(&self) -> bool {
        windows::supports_windows_update(self)
    }

    pub fn windows_update(&self, ctx: &ExecutionContext) -> Result<()> {
        windows::windows_update(self, ctx)
    }

    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        windows::microsoft_store(self, ctx)
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    pub fn supports_windows_update(powershell: &Powershell) -> bool {
        powershell
            .path
            .as_ref()
            .map(|p| has_module(p, "PSWindowsUpdate"))
            .unwrap_or(false)
    }

    pub fn windows_update(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        debug_assert!(supports_windows_update(powershell));

        let mut args = vec!["Install-WindowsUpdate", "-Verbose"];
        if ctx.config().accept_all_windows_updates() {
            args.push("-AcceptAll");
        }

        powershell.build_command_internal(ctx, &args)?.status_checked()
    }

    pub fn microsoft_store(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        println!("{}", t!("Scanning for updates..."));
        let update_command = "(Get-CimInstance -Namespace \"Root\\cimv2\\mdm\\dmmap\" \
                                 -ClassName \"MDM_EnterpriseModernAppManagement_AppManagement01\" | \
                                 Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue";
        powershell
            .build_command_internal(ctx, &["-Command", update_command])?
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

    fn has_module(powershell: &PathBuf, command: &str) -> bool {
        Command::new(powershell)
            .args([
                "-NoProfile",
                "-Command",
                &format!("Get-Module -ListAvailable {}", command),
            ])
            .output_checked_utf8()
            .map(|result| !result.stdout.is_empty())
            .unwrap_or(false)
    }
}
