#[cfg(windows)]
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::Result;
use rust_i18n::t;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::{is_dumb, print_separator};
use crate::utils::{require_option, which, PathExt};
use crate::Step;

pub struct Powershell {
    path: Option<PathBuf>,
    profile: Option<PathBuf>,
}

impl Powershell {
    /// Returns a powershell instance.
    ///
    /// If the powershell binary is not found, or the current terminal is dumb
    /// then the instance of this struct will skip all the powershell steps.
    pub fn new() -> Self {
        let path = which("pwsh").or_else(|| which("powershell")).filter(|_| !is_dumb());

        let profile = path.as_ref().and_then(|path| {
            Command::new(path)
                .args(["-NoProfile", "-Command", "Split-Path $profile"])
                .output_checked_utf8()
                .map(|output| PathBuf::from(output.stdout.trim()))
                .and_then(|p| p.require())
                .ok()
        });

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

    pub fn profile(&self) -> Option<&PathBuf> {
        self.profile.as_ref()
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;

        print_separator(t!("Powershell Modules Update"));

        let unload_cmd = ["Get-Module | Remove-Module -Force"];
        let mut update_cmd = vec!["Update-Module"];
        let reload_cmd = ["Get-Module -ListAvailable | Import-Module"];

        if ctx.config().verbose() {
            update_cmd.push("-Verbose");
        }

        if ctx.config().yes(Step::Powershell) {
            update_cmd.push("-Force");
        }

        println!("{}", t!("Unloading modules..."));
        ctx.run_type()
            .execute(powershell)
            .args(["-NoProfile", "-Command", &unload_cmd.join(" ")])
            .status_checked()?;

        println!("{}", t!("Updating modules..."));
        ctx.run_type()
            .execute(powershell)
            .args(["-NoProfile", "-Command", &update_cmd.join(" ")])
            .status_checked()?;

        println!("{}", t!("Reloading modules..."));
        ctx.run_type()
            .execute(powershell)
            .args(["-NoProfile", "-Command", &reload_cmd.join(" ")])
            .status_checked()
    }

    #[cfg(windows)]
    pub fn supports_windows_update(&self) -> bool {
        self.path
            .as_ref()
            .map(|p| Self::has_module(p, "PSWindowsUpdate"))
            .unwrap_or(false)
    }

    #[cfg(windows)]
    pub fn windows_update(&self, ctx: &ExecutionContext) -> Result<()> {
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;

        debug_assert!(self.supports_windows_update());

        let accept_all = if ctx.config().accept_all_windows_updates() {
            "-AcceptAll"
        } else {
            ""
        };

        let install_windowsupdate_verbose = "Install-WindowsUpdate -Verbose".to_string();

        let mut command = if let Some(sudo) = ctx.sudo() {
            let mut command = ctx.run_type().execute(sudo);
            command.arg(powershell);
            command
        } else {
            ctx.run_type().execute(powershell)
        };

        command
            .args(["-NoProfile", &install_windowsupdate_verbose, accept_all])
            .status_checked()
    }

    #[cfg(windows)]
    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;

        let mut command = if let Some(sudo) = ctx.sudo() {
            let mut command = ctx.run_type().execute(sudo);
            command.arg(powershell);
            command
        } else {
            ctx.run_type().execute(powershell)
        };

        println!("{}", t!("Scanning for updates..."));

        // Scan for updates using the MDM UpdateScanMethod
        // This method is also available for non-MDM devices
        let update_command = "(Get-CimInstance -Namespace \"Root\\cimv2\\mdm\\dmmap\" -ClassName \"MDM_EnterpriseModernAppManagement_AppManagement01\" | Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue";

        command.args(["-NoProfile", update_command]);

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
}
