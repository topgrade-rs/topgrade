#[cfg(windows)]
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

const NO_PROFILE: &str = "-NoProfile";
const REMOTE_SIGNED: &str = "RemoteSigned";
const POWERSHELL_NOT_INSTALLED: &str = "Powershell is not installed";

pub struct Powershell {
    path: Option<PathBuf>,
    profile: Option<PathBuf>,
}

impl Powershell {
    pub fn new() -> Self {
        let path = which("pwsh").or_else(|| which("powershell")).filter(|_| !is_dumb());
        let profile = path.as_ref().and_then(|path| Self::get_profile_path(path));
        Powershell { path, profile }
    }

    #[cfg(windows)]
    pub fn windows_powershell() -> Self {
        Powershell {
            path: which("powershell").filter(|_| !is_dumb()),
            profile: None,
        }
    }

    #[cfg(windows)]
    pub fn has_module(powershell: &Path, command: &str) -> bool {
        Self::run_command(powershell, &["-Command", &format!("Get-Module -ListAvailable {command}")])
            .map(|result| !result.stdout.is_empty())
            .unwrap_or(false)
    }

    pub fn profile(&self) -> Option<&PathBuf> {
        self.profile.as_ref()
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        let powershell = require_option(self.path.as_ref(), t!(POWERSHELL_NOT_INSTALLED).to_string())?;
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

    #[cfg(windows)]
    pub fn supports_windows_update(&self) -> bool {
        self.path
            .as_ref()
            .map(|p| Self::has_module(p, "PSWindowsUpdate"))
            .unwrap_or(false)
    }

    #[cfg(windows)]
    fn execution_policy_args_if_needed(&self) -> Option<&'static [&'static str]> {
        if self.is_execution_policy_set(REMOTE_SIGNED) {
            None
        } else {
            Some(&["-ExecutionPolicy", REMOTE_SIGNED, "-Scope", "Process"])
        }
    }

    #[cfg(windows)]
    fn is_execution_policy_set(&self, policy: &str) -> bool {
        self.path.as_ref().map_or(false, |powershell| {
            Self::run_command(powershell, &["-Command", "Get-ExecutionPolicy -Scope Process"])
                .map(|output| output.stdout.trim() == policy)
                .unwrap_or(false)
        })
    }

    #[cfg(windows)]
    fn common_args() -> &'static [&'static str] {
        &[NO_PROFILE]
    }

    #[cfg(windows)]
    pub fn windows_update(&self, ctx: &ExecutionContext) -> Result<()> {
        let powershell = require_option(self.path.as_ref(), t!(POWERSHELL_NOT_INSTALLED).to_string())?;
        debug_assert!(self.supports_windows_update());

        let accept_all = if ctx.config().accept_all_windows_updates() {
            "-AcceptAll"
        } else {
            ""
        };

        let install_windowsupdate_verbose = "Install-WindowsUpdate -Verbose".to_string();
        self.execute_command(ctx, powershell, &[&install_windowsupdate_verbose, accept_all])
    }

    #[cfg(windows)]
    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        let powershell = require_option(self.path.as_ref(), t!(POWERSHELL_NOT_INSTALLED).to_string())?;
        println!("{}", t!("Scanning for updates..."));

        let update_command = "(Get-CimInstance -Namespace \"Root\\cimv2\\mdm\\dmmap\" -ClassName \"MDM_EnterpriseModernAppManagement_AppManagement01\" | Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue";
        self.execute_command(ctx, powershell, &[update_command])
            .and_then(|output| {
                if output.stdout.trim() == "0" {
                    println!("{}", t!("Success, Microsoft Store apps are being updated in the background"));
                    Ok(())
                } else {
                    println!("{}", t!("Unable to update Microsoft Store apps, manual intervention is required"));
                    Err(())
                }
            })
    }

    fn get_profile_path(path: &Path) -> Option<PathBuf> {
        Command::new(path)
            .args([NO_PROFILE, "-Command", "Split-Path $profile"])
            .output_checked_utf8()
            .map(|output| PathBuf::from(output.stdout.trim()))
            .and_then(super::super::utils::PathExt::require)
            .ok()
    }

    fn run_command(powershell: &Path, args: &[&str]) -> Result<CommandOutput> {
        Command::new(powershell)
            .args(Self::common_args())
            .args(args)
            .output_checked_utf8()
    }

    fn execute_command(&self, ctx: &ExecutionContext, powershell: &Path, cmd: &[&str]) -> Result<CommandOutput> {
        let mut command = if let Some(sudo) = ctx.sudo() {
            let mut command = ctx.run_type().execute(sudo);
            command.arg(powershell);
            command
        } else {
            ctx.run_type().execute(powershell)
        };

        if let Some(args) = self.execution_policy_args_if_needed() {
            command.args(args);
        }

        command.args(Self::common_args()).args(cmd).status_checked()
    }
}