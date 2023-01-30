#[cfg(windows)]
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::Result;

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
        let powershell = require_option(self.path.as_ref(), String::from("Powershell is not installed"))?;

        print_separator("Powershell Modules Update");

        let mut cmd = vec!["Update-Module"];

        if ctx.config().verbose() {
            cmd.push("-Verbose")
        }

        if ctx.config().yes(Step::Powershell) {
            cmd.push("-Force")
        }

        println!("Updating modules...");
        ctx.run_type()
            .execute(powershell)
            // This probably doesn't need `shell_words::join`.
            .args(["-NoProfile", "-Command", &cmd.join(" ")])
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
        let powershell = require_option(self.path.as_ref(), String::from("Powershell is not installed"))?;

        debug_assert!(self.supports_windows_update());

        let mut command = if let Some(sudo) = ctx.sudo() {
            let mut command = ctx.run_type().execute(sudo);
            command.arg(powershell);
            command
        } else {
            ctx.run_type().execute(powershell)
        };

        command
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Import-Module PSWindowsUpdate; Install-WindowsUpdate -MicrosoftUpdate {} -Verbose",
                    if ctx.config().accept_all_windows_updates() {
                        "-AcceptAll"
                    } else {
                        ""
                    }
                ),
            ])
            .status_checked()
    }
}
