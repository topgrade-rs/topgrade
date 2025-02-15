use std::path::Path;
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

    pub fn profile(&self) -> Option<&PathBuf> {
        self.profile.as_ref()
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;
        print_separator(t!("Powershell Modules Update"));

        let mut cmd = vec!["Update-Module"];
        if ctx.config().verbose() {
            cmd.push("-Verbose");
        }
        if ctx.config().yes(Step::Powershell) {
            cmd.push("-Force");
        }

        println!("{}", t!("Updating modules..."));
        self.execute_command(ctx, powershell, &cmd)
    }

    fn execute_command(&self, ctx: &ExecutionContext, powershell: &Path, cmd: &[&str]) -> Result<()> {
        ctx.run_type()
            .execute(powershell)
            .args(Self::common_args())
            .args(["-Command", &cmd.join(" ")])
            .status_checked()
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
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;
        debug_assert!(self.supports_windows_update());

        let install_windowsupdate_verbose = "Install-WindowsUpdate -Verbose".to_string();
        let mut command = self.prepare_command(ctx, powershell);

        if let Some(args) = self.execution_policy_args_if_needed() {
            command.args(args);
        }

        command.args(Self::common_args());
        command.arg(&install_windowsupdate_verbose);

        if ctx.config().accept_all_windows_updates() {
            command.arg("-AcceptAll");
        }

        command.status_checked()
    }

    #[cfg(windows)]
    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;
        let mut command = self.prepare_command(ctx, powershell);

        println!("{}", t!("Scanning for updates..."));

        let update_command = "(Get-CimInstance -Namespace \"Root\\cimv2\\mdm\\dmmap\" -ClassName \"MDM_EnterpriseModernAppManagement_AppManagement01\" | Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue";

        if let Some(args) = self.execution_policy_args_if_needed() {
            command.args(args);
        }

        command.args(Self::common_args()).args([update_command]);

        command
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
    fn prepare_command(&self, ctx: &ExecutionContext, powershell: &Path) -> Command {
        if let Some(sudo) = ctx.sudo() {
            let mut command = ctx.run_type().execute(sudo);
            command.arg(powershell);
            command
        } else {
            ctx.run_type().execute(powershell)
        }
    }

    #[cfg(windows)]
    fn has_module(powershell: &Path, command: &str) -> bool {
        Command::new(powershell)
            .args([
                "-NoProfile",
                "-Command",
                &format!("Get-Module -ListAvailable {command}"),
            ])
            .output_checked_utf8()
            .map(|result| !result.stdout.is_empty())
            .unwrap_or(false)
    }
}
