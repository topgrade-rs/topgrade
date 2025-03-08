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

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;

        print_separator(t!("Powershell Modules Update"));

        // Combine all commands into a single script
        let mut script_commands = Vec::new();

        // Unload modules
        script_commands.push(concat!(
            "Write-Host \"Unloading modules...\" -ForegroundColor Yellow\n",
            "Get-Module | ForEach-Object {\n",
            "  $moduleName = $_.Name\n",
            "  Write-Host \"Unloading module: $moduleName\" -ForegroundColor Yellow\n",
            "  Remove-Module -Name $moduleName -Force\n",
            "}"
        ));

        // Update modules
        let mut update_cmd = vec![
            "Write-Host \"Updating modules...\" -ForegroundColor Cyan",
            "Get-Module -ListAvailable | Select-Object -Property Name -Unique | ForEach-Object {",
            "  $moduleName = $_.Name",
            "  try {",
            "    # Check if module was installed via Install-Module before attempting to update",
            "    if (Get-InstalledModule -Name $moduleName -ErrorAction SilentlyContinue) {",
            "      Write-Host \"Updating module: $moduleName\" -ForegroundColor Cyan",
            "      Update-Module -Name $moduleName",
        ];

        if ctx.config().verbose() {
            update_cmd.push("      -Verbose");
        }

        if ctx.config().yes(Step::Powershell) {
            update_cmd.push("      -Force");
        }

        update_cmd.extend_from_slice(&[
            "    } else {",
            "      Write-Host \"Skipping module: $moduleName (not installed via Install-Module)\" -ForegroundColor Yellow",
            "    }",
            "  } catch {",
            "    Write-Host \"Failed to update module: $moduleName - $($_.Exception.Message)\" -ForegroundColor Red",
            "  }",
            "}",
        ]);
        script_commands.push(update_cmd.join("\n"));

        // Reload modules
        script_commands.push(concat!(
            "Write-Host \"Reloading modules...\" -ForegroundColor Green\n",
            "Get-Module -ListAvailable | ForEach-Object {\n",
            "  if (Test-Path $_.ModuleBase) {\n",
            "    try {\n",
            "      Import-Module $_.Name -ErrorAction Stop\n",
            "      Write-Host \"Successfully imported module: $($_.Name)\" -ForegroundColor Green\n",
            "    } catch {\n",
            "      # Silently ignore import failures - these are often expected for modules with dependencies\n",
            "      # or modules requiring specific PowerShell hosts\n",
            "    }\n",
            "  }\n",
            "}"
        ));

        // Join all commands with semicolons for a single execution
        let full_script = script_commands.join(";\n\n");

        // Execute the combined script with a single elevation request
        #[cfg(windows)]
        {
            let mut cmd = if let Some(sudo) = ctx.sudo() {
                let mut cmd = ctx.run_type().execute(sudo);
                cmd.arg(&powershell);
                cmd
            } else {
                ctx.run_type().execute(&powershell)
            };
            return cmd.args(["-NoProfile", "-Command", &full_script]).status_checked();
        }

        #[cfg(not(windows))]
        ctx.run_type()
            .execute(&powershell)
            .args(["-NoProfile", "-Command", &full_script])
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
