#[cfg(windows)]
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::Result;
use rust_i18n::t;

use crate::command::CommandExt;
use crate::config::Step;
use crate::execution_context::ExecutionContext;
use crate::executor::Executor;
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
        let profile = Self::find_profile(&path);
        Powershell { path, profile }
    }

    /// Helper to find the PowerShell profile path
    fn find_profile(path: &Option<PathBuf>) -> Option<PathBuf> {
        path.as_ref().and_then(|path| {
            Command::new(path)
                .args(Self::default_args())
                .arg("-Command")
                .arg("Split-Path $profile")
                .output_checked_utf8()
                .map(|output| PathBuf::from(output.stdout.trim()))
                .and_then(super::super::utils::PathExt::require)
                .ok()
        })
    }

    /// Returns the default PowerShell command arguments used in most commands
    fn default_args() -> [&'static str; 3] {
        ["-NoProfile", "-NoLogo", "-NonInteractive"]
    }

    /// Helper to run PowerShell with a command
    #[cfg(windows)]
    fn run_ps_command(&self, path: &Path, command: &str) -> Result<String> {
        Command::new(path)
            .args(Self::default_args())
            .arg("-Command")
            .arg(command)
            .output_checked_utf8()
            .map(|output| output.stdout)
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
            .args(Self::default_args())
            .arg("-Command")
            .arg(format!("Get-Module -ListAvailable {command}"))
            .output_checked_utf8()
            .map(|result| !result.stdout.is_empty())
            .unwrap_or(false)
    }

    pub fn profile(&self) -> Option<&PathBuf> {
        self.profile.as_ref()
    }

    /// Creates the PowerShell script for updating modules
    fn create_update_script(&self, ctx: &ExecutionContext) -> String {
        let force_flag = self.get_force_flag(ctx);
        let update_command = self.build_update_command(force_flag, ctx.config().verbose());

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
      
      {}
      
      # Update the module
      Write-Host "  {}" -ForegroundColor Cyan
      {}
      
      {}
    }}
  }} catch {{
    Write-Host "{}" -ForegroundColor Red
  }}
}}
Write-Host "{}" -ForegroundColor Green"#,
            t!("Processing PowerShell modules..."),
            t!("Processing module: {moduleName}", moduleName = "$moduleName"),
            self.generate_module_unload_script(),
            t!("Updating module: {moduleName}", moduleName = "$moduleName"),
            update_command,
            self.generate_module_reload_script(),
            t!(
                "Failed to process module: {moduleName} - {error}",
                moduleName = "$moduleName",
                error = "$($_.Exception.Message)"
            ),
            t!("PowerShell module processing complete.")
        )
    }

    /// Helper to get the force flag based on config
    fn get_force_flag(&self, ctx: &ExecutionContext) -> &str {
        if ctx.config().yes(Step::Powershell) || ctx.config().powershell_force_modules_update() {
            " -Force"
        } else {
            ""
        }
    }

    /// Helper to build the update command with appropriate options
    fn build_update_command(&self, force_flag: &str, verbose: bool) -> String {
        if verbose {
            format!("Update-Module -Name $moduleName -Verbose{}", force_flag)
        } else {
            format!("Update-Module -Name $moduleName{}", force_flag)
        }
    }

    /// Generate the script for unloading a module
    fn generate_module_unload_script(&self) -> String {
        format!(
            r#"# Check if the module is loaded and unload it if necessary
      Write-Host "  {}" -ForegroundColor Yellow
      if (Get-Module -Name $moduleName -ErrorAction SilentlyContinue) {{
        Remove-Module -Name $moduleName -Force -ErrorAction SilentlyContinue
      }} else {{
        Write-Host "    Module is not currently loaded" -ForegroundColor Yellow
      }}"#,
            t!("Unloading module: {moduleName}", moduleName = "$moduleName")
        )
    }

    /// Generate the script for reloading a module
    fn generate_module_reload_script(&self) -> String {
        format!(
            r#"# Reload the module
      try {{
        Write-Host "  {}" -ForegroundColor Green
        Import-Module $moduleName -ErrorAction Stop
        Write-Host "  {}" -ForegroundColor Green
      }} catch {{
        Write-Host "  {}" -ForegroundColor Yellow
      }}"#,
            t!("Reloading module: {moduleName}", moduleName = "$moduleName"),
            t!("Successfully imported module: {moduleName}", moduleName = "$moduleName"),
            t!(
                "Could not reload module: {moduleName} - {error}",
                moduleName = "$moduleName",
                error = "$($_.Exception.Message)"
            )
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

    /// Execute a PowerShell script with standard arguments
    fn execute_script(&self, ctx: &ExecutionContext, script: &str) -> Result<()> {
        let mut cmd = self.create_powershell_command(ctx)?;
        cmd.args(Self::default_args())
            .arg("-Command")
            .arg(script)
            .status_checked()
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("Powershell Modules Update"));
        let script = self.create_update_script(ctx);
        self.execute_script(ctx, &script)
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

        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;

        let accept_all = if ctx.config().accept_all_windows_updates() {
            "-AcceptAll"
        } else {
            ""
        };

        let install_command = format!(
            "Install-WindowsUpdate {} {}",
            if ctx.config().verbose() { "-Verbose" } else { "" },
            accept_all
        );

        // Use the run_ps_command helper method
        self.run_ps_command(powershell, &install_command)?;
        Ok(())
    }

    #[cfg(windows)]
    pub fn microsoft_store(&self, _ctx: &ExecutionContext) -> Result<()> {
        println!("{}", t!("Scanning for updates..."));

        // Get powershell path
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;

        // Scan for updates using the MDM UpdateScanMethod
        let update_command = self.build_microsoft_store_update_command();

        // Use the run_ps_command helper method
        let output = self.run_ps_command(powershell, &update_command)?;

        // Process the result
        if output.trim() == "0" {
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
            Err(color_eyre::eyre::eyre!("Microsoft Store update failed"))
        }
    }

    #[cfg(windows)]
    fn build_microsoft_store_update_command(&self) -> String {
        "(Get-CimInstance -Namespace \"Root\\cimv2\\mdm\\dmmap\" -ClassName \"MDM_EnterpriseModernAppManagement_AppManagement01\" | Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue".to_string()
    }
}
