#[cfg(windows)]
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::Result;
use rust_i18n::t;

use crate::command::CommandExt;
use crate::config::Step;
use crate::execution_context::ExecutionContext;
use crate::executor::Executor; // Added this import
use crate::terminal::{is_dumb, print_separator};
use crate::utils::{require_option, which};

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
                .and_then(super::super::utils::PathExt::require)
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

    /// Creates the PowerShell script for updating modules
    fn create_update_script(&self, ctx: &ExecutionContext) -> String {
        // Determine if we should use -Force based on config
        let force_flag = if ctx.config().yes(Step::Powershell) || ctx.config().powershell_force_modules_update() {
            " -Force"
        } else {
            ""
        };

        // Build the update command with or without verbosity
        let update_command = if ctx.config().verbose() {
            format!("Update-Module -Name $moduleName -Verbose{}", force_flag)
        } else {
            format!("Update-Module -Name $moduleName{}", force_flag)
        };

        // Format the entire script using a template style for better readability
        format!(
            r#"Write-Host "{}" -ForegroundColor Cyan
Get-Module -ListAvailable | Select-Object -Property Name -Unique | ForEach-Object {{
  $moduleName = $_.Name
  try {{
    # Only process modules installed via Install-Module
    if (Get-InstalledModule -Name $moduleName -ErrorAction SilentlyContinue) {{
      # Process each module individually - unload, update, reload
      Write-Host "{}" -ForegroundColor Cyan
      
      # Check if the module is loaded and unload it if necessary
      Write-Host "  {}" -ForegroundColor Yellow
      if (Get-Module -Name $moduleName -ErrorAction SilentlyContinue) {{
        Remove-Module -Name $moduleName -Force -ErrorAction SilentlyContinue
      }} else {{
        Write-Host "    Module is not currently loaded" -ForegroundColor Yellow
      }}
      
      # Update the module
      Write-Host "  {}" -ForegroundColor Cyan
      {}
      
      # Reload the module
      try {{
        Write-Host "  {}" -ForegroundColor Green
        Import-Module $moduleName -ErrorAction Stop
        Write-Host "  {}" -ForegroundColor Green
      }} catch {{
        Write-Host "  {}" -ForegroundColor Yellow
      }}
    }}
  }} catch {{
    Write-Host "{}" -ForegroundColor Red
  }}
}}
Write-Host "{}" -ForegroundColor Green"#,
            t!("Processing PowerShell modules..."),
            t!("Processing module: {moduleName}", moduleName = "$moduleName"),
            t!("Unloading module: {moduleName}", moduleName = "$moduleName"),
            t!("Updating module: {moduleName}", moduleName = "$moduleName"),
            update_command,
            t!("Reloading module: {moduleName}", moduleName = "$moduleName"),
            t!("Successfully imported module: {moduleName}", moduleName = "$moduleName"),
            t!(
                "Could not reload module: {moduleName} - {error}",
                moduleName = "$moduleName",
                error = "$($_.Exception.Message)"
            ),
            t!(
                "Failed to process module: {moduleName} - {error}",
                moduleName = "$moduleName",
                error = "$($_.Exception.Message)"
            ),
            t!("PowerShell module processing complete.")
        )
    }

    /// Creates a command to execute PowerShell with optional sudo elevation
    fn create_powershell_command(&self, ctx: &ExecutionContext) -> Result<Executor> {
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;

        let cmd = if let Some(sudo) = ctx.sudo() {
            let mut cmd = ctx.run_type().execute(sudo);
            cmd.arg(powershell);
            cmd
        } else {
            ctx.run_type().execute(powershell)
        };

        Ok(cmd)
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("Powershell Modules Update"));

        // Create the update script using the dedicated function
        let script = self.create_update_script(ctx);

        let mut cmd = self.create_powershell_command(ctx)?;
        cmd.args(["-NoProfile", "-NoLogo", "-NonInteractive", "-Command", &script])
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
        debug_assert!(self.supports_windows_update());

        let accept_all = if ctx.config().accept_all_windows_updates() {
            "-AcceptAll"
        } else {
            ""
        };

        let install_windowsupdate_verbose = "Install-WindowsUpdate -Verbose".to_string();

        let mut cmd = self.create_powershell_command(ctx)?;
        cmd.args(["-NoProfile", &install_windowsupdate_verbose, accept_all])
            .status_checked()
    }

    #[cfg(windows)]
    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        let mut cmd = self.create_powershell_command(ctx)?;

        println!("{}", t!("Scanning for updates..."));

        // Scan for updates using the MDM UpdateScanMethod
        // This method is also available for non-MDM devices
        let update_command = "(Get-CimInstance -Namespace \"Root\\cimv2\\mdm\\dmmap\" -ClassName \"MDM_EnterpriseModernAppManagement_AppManagement01\" | Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue";

        cmd.args(["-NoProfile", update_command]);

        cmd.output_checked_with_utf8(|output| {
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
