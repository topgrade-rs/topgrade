use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
use color_eyre::eyre::eyre;
use color_eyre::eyre::Result;
use rust_i18n::t;
use tracing::debug;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::{is_dumb, print_separator};
use crate::utils::{require_option, which, PathExt};

pub struct Powershell {
    path: Option<PathBuf>,
    profile: Option<PathBuf>,
}

impl Powershell {
    pub fn new() -> Self {
        let path = which("pwsh").or_else(|| which("powershell")).filter(|_| !is_dumb());
        let profile = path.as_ref().and_then(Self::get_profile);
        Self { path, profile }
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
        let profile = Self::build_command_internal(path, "Split-Path $PROFILE")
            .output_checked_utf8()
            .map(|output| output.stdout.trim().to_string())
            .and_then(|s| PathBuf::from(s).require())
            .ok();
        debug!("Found PowerShell profile: {:?}", profile);
        profile
    }

    /// Builds an "internal" powershell command
    fn build_command_internal(path: &PathBuf, cmd: &str) -> Command {
        let mut command = Command::new(path);

        command.args(["-NoProfile", "-Command"]);
        command.arg(cmd);

        command
    }

    /// Builds a "primary" powershell command (uses dry-run if required):
    /// {powershell} -NoProfile -Command {cmd}
    fn build_command<'a>(&self, ctx: &'a ExecutionContext, cmd: &str) -> Result<impl CommandExt + 'a> {
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

        command.args(["-NoProfile", "-Command"]);
        command.arg(cmd);

        Ok(command)
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("Powershell Modules Update"));

        let mut cmd = "Update-Module".to_string();

        if ctx.config().verbose() {
            cmd.push_str(" -Verbose");
        }
        if ctx.config().yes(Step::Powershell) {
            cmd.push_str(" -Force");
        }

        println!("{}", t!("Updating modules..."));

        self.build_command(ctx, &cmd)?.status_checked()
    }

    #[cfg(windows)]
    pub fn execution_policy_args_if_needed(&self) -> Result<()> {
        if !self.is_execution_policy_set("RemoteSigned") {
            Err(eyre!(
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

            let mut command = Self::build_command_internal(powershell, "Get-ExecutionPolicy");

            let current_policy = command
                .output_checked_utf8()
                .map(|output| output.stdout.trim().to_string());

            debug!("Found PowerShell ExecutionPolicy: {:?}", current_policy);

            if let Ok(current_policy) = current_policy {
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
    fn has_module(&self, module_name: &str) -> bool {
        if let Some(powershell) = &self.path {
            let cmd = format!("Get-Module -ListAvailable {}", module_name);

            return Self::build_command_internal(powershell, &cmd)
                .output_checked()
                .map(|output| !output.stdout.trim_ascii().is_empty())
                .unwrap_or(false);
        }
        false
    }

    pub fn supports_windows_update(&self) -> bool {
        self.has_module("PSWindowsUpdate")
    }

    pub fn windows_update(&self, ctx: &ExecutionContext) -> Result<()> {
        use crate::config::UpdatesAutoReboot;

        debug_assert!(self.supports_windows_update());

        let mut cmd = "Import-Module PSWindowsUpdate; Install-WindowsUpdate -Verbose".to_string();

        if ctx.config().accept_all_windows_updates() {
            cmd.push_str(" -AcceptAll");
        }

        match ctx.config().windows_updates_auto_reboot() {
            UpdatesAutoReboot::Yes => cmd.push_str(" -AutoReboot"),
            UpdatesAutoReboot::No => cmd.push_str(" -IgnoreReboot"),
            UpdatesAutoReboot::Ask => (), // Prompting is the default for Install-WindowsUpdate
        }

        self.build_command(ctx, &cmd)?.status_checked()
    }

    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        println!("{}", t!("Scanning for updates..."));
        let cmd = "Start-Process powershell -Verb RunAs -ArgumentList '-Command', \
            '(Get-CimInstance -Namespace \"Root\\cimv2\\mdm\\dmmap\" \
            -ClassName \"MDM_EnterpriseModernAppManagement_AppManagement01\" | \
            Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue'";

        self.build_command(ctx, cmd)?.status_checked()
    }
}
