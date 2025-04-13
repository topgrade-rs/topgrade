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
            // Check execution policy and return early if it's not set correctly
            self.execution_policy_args_if_needed()?;
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
    pub fn execution_policy_args_if_needed(&self) -> Result<()> {
        if !self.is_execution_policy_set("RemoteSigned") {
            Err(color_eyre::eyre::eyre!(
                "PowerShell execution policy is too restrictive. \
                Please run 'Set-ExecutionPolicy RemoteSigned -Scope CurrentUser' in PowerShell \
                (or use Unrestricted/Bypass if you're sure about the security implications)"
            ))
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    fn is_execution_policy_set(&self, policy: &str) -> bool {
        if let Some(powershell) = &self.path {
            // These policies are ordered from most restrictive to least restrictive
            let valid_policies = ["Restricted", "AllSigned", "RemoteSigned", "Unrestricted", "Bypass"];

            // Find the index of our target policy
            let target_idx = valid_policies.iter().position(|&p| p == policy);

            let output = Command::new(powershell)
                .args(["-NoProfile", "-Command", "Get-ExecutionPolicy"])
                .output_checked_utf8();

            if let Ok(output) = output {
                let current_policy = output.stdout.trim();

                // Find the index of the current policy
                let current_idx = valid_policies.iter().position(|&p| p == current_policy);

                // Check if current policy exists and is at least as permissive as the target
                return match (current_idx, target_idx) {
                    (Some(current), Some(target)) => current >= target,
                    _ => false,
                };
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

    #[cfg(windows)]
    pub fn windows_update(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        debug_assert!(supports_windows_update(powershell));

        // Build the full command string
        let mut command_str = "Install-WindowsUpdate -Verbose".to_string();
        if ctx.config().accept_all_windows_updates() {
            command_str.push_str(" -AcceptAll");
        }

        // Pass the command string using the -Command flag
        powershell
            .build_command_internal(ctx, &["-Command", &command_str])?
            .status_checked()
    }

    pub fn microsoft_store(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        println!("{}", t!("Scanning for updates..."));
        let update_command = "Start-Process powershell -Verb RunAs -ArgumentList '-Command', \
            '(Get-CimInstance -Namespace \"Root\\cimv2\\mdm\\dmmap\" \
            -ClassName \"MDM_EnterpriseModernAppManagement_AppManagement01\" | \
            Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue'";

        powershell
            .build_command_internal(ctx, &["-Command", update_command])?
            .status_checked()
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
