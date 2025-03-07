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

        // Define core system modules that need special handling
        let system_modules_update_script = r#"
# List of system/core modules that need special handling
$systemModules = @("PSReadLine", "Microsoft.PowerShell.Host", "PowerShellGet", "PackageManagement")

# Get information about system modules
$systemModuleInfo = $systemModules | ForEach-Object {
    $currentModule = $_
    try {
        $installedVersions = Get-Module -Name $currentModule -ListAvailable | 
            Select-Object -ExpandProperty Version | 
            Sort-Object -Descending | 
            Select-Object -First 1
        
        $onlineInfo = Find-Module -Name $currentModule -ErrorAction SilentlyContinue
        
        if ($onlineInfo -and ($onlineInfo.Version -gt $installedVersions)) {
            [PSCustomObject]@{
                Name = $currentModule
                InstalledVersion = $installedVersions
                OnlineVersion = $onlineInfo.Version
                NeedsUpdate = $true
            }
        } else {
            [PSCustomObject]@{
                Name = $currentModule
                InstalledVersion = $installedVersions
                OnlineVersion = $onlineInfo.Version
                NeedsUpdate = $false
            }
        }
    } catch {
        Write-Host "Error checking module $currentModule`: $_" -ForegroundColor Yellow
        $null
    }
} | Where-Object { $_ -ne $null }

# Display information about system modules
Write-Host "System modules status:" -ForegroundColor Cyan
$systemModuleInfo | Format-Table -AutoSize

# Ask if the user wants to update system modules (this will require restart)
$systemModulesToUpdate = $systemModuleInfo | Where-Object { $_.NeedsUpdate }
if ($systemModulesToUpdate) {
    Write-Host "The following system modules can be updated, but will require restarting PowerShell:" -ForegroundColor Yellow
    $systemModulesToUpdate | Format-Table Name, InstalledVersion, OnlineVersion -AutoSize
    
    Write-Host "System modules will be updated separately after regular module updates." -ForegroundColor Cyan
}
"#;

        // Regular module update script
        let regular_update_script = r#"
# Set error preferences
$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'  # Hide progress bars for faster execution

# First try to unload non-critical modules
Write-Host "Unloading non-critical modules..." -ForegroundColor Cyan
Get-Module | Where-Object { 
    $_.Name -notin @("PSReadLine", "Microsoft.PowerShell.Host", "PowerShellGet", "PackageManagement") 
} | ForEach-Object {
    try {
        Remove-Module -Name $_.Name -Force -ErrorAction Stop
        Write-Verbose "Successfully unloaded module: $($_.Name)"
    } catch {
        Write-Verbose "Could not unload module: $($_.Name) - $($_.Exception.Message)"
    }
}

# Get list of installed regular modules (excluding system modules)
Write-Host "Scanning for updatable modules..." -ForegroundColor Cyan
$regularModules = Get-Module -ListAvailable | Where-Object { 
    $_.Name -notin @("PSReadLine", "Microsoft.PowerShell.Host", "PowerShellGet", "PackageManagement") 
} | Group-Object -Property Name | ForEach-Object { 
    $_.Group | Sort-Object Version -Descending | Select-Object -First 1 
}

# Update regular modules
$updated = 0
$failed = 0
$skipped = 0
$total = $regularModules.Count

Write-Host "Found $total non-system modules to process" -ForegroundColor Cyan

foreach ($module in $regularModules) {
    $moduleName = $module.Name
    Write-Host "Processing [$($updated+$failed+$skipped+1)/$total] $moduleName... " -NoNewline
    
    try {
        # Check for permissions or special modules we should skip
        if ($moduleName -eq "PSCompletions" -or $moduleName -like "Az.*" -or
            $moduleName -eq "CompletionPredictor") {
            Write-Host "SKIPPED (special module)" -ForegroundColor Yellow
            $skipped++
            continue
        }
        
        # Try to update the module
        Update-Module -Name $moduleName -Force -ErrorAction Stop
        Write-Host "UPDATED" -ForegroundColor Green
        $updated++
    }
    catch {
        # Check if it's just because the module is already at the latest version
        if ($_.Exception.Message -like "*because no newer module exists*") {
            Write-Host "OK (latest version)" -ForegroundColor Green
            $updated++ # Count as success
        }
        else {
            Write-Host "FAILED - $($_.Exception.Message)" -ForegroundColor Red
            $failed++
        }
    }
}

# Show summary for regular modules
Write-Host "`nRegular module update summary:" -ForegroundColor Cyan
Write-Host "  Updated/Current: $updated" -ForegroundColor Green
if ($failed -gt 0) {
    Write-Host "  Failed: $failed" -ForegroundColor Red
}
if ($skipped -gt 0) {
    Write-Host "  Skipped: $skipped" -ForegroundColor Yellow
}
"#;

        // System modules update script (if needed will be run separately)
        let system_update_script = r#"
# Update system modules that need updates
$updatedSystem = 0
$failedSystem = 0

foreach ($module in $systemModulesToUpdate) {
    $moduleName = $module.Name
    Write-Host "Updating system module $moduleName... " -NoNewline
    
    try {
        # Special handling for critical system modules
        # For these, we use Install-Module with -Force and -AllowClobber
        # which is more reliable than Update-Module for system components
        Install-Module -Name $moduleName -Force -AllowClobber -SkipPublisherCheck -ErrorAction Stop
        Write-Host "UPDATED" -ForegroundColor Green
        $updatedSystem++
    }
    catch {
        Write-Host "FAILED - $($_.Exception.Message)" -ForegroundColor Red
        $failedSystem++
    }
}

if ($updatedSystem -gt 0) {
    Write-Host "`nSystem modules were updated. You should restart PowerShell to use the new versions." -ForegroundColor Yellow
}
"#;

        // Final reloading script
        let reload_script = r#"
# Reload regular modules, avoiding problematic ones
Write-Host "Reloading modules..." -ForegroundColor Cyan
Get-Module -ListAvailable | Where-Object {
    # Skip system modules (they're already loaded) and problematic modules
    $_.Name -notin @(
        "PSReadLine", "Microsoft.PowerShell.Host", "PowerShellGet", "PackageManagement",
        "PSCompletions", "CompletionPredictor"
    )
} | Sort-Object -Property Name -Unique | ForEach-Object {
    try {
        Import-Module $_.Name -DisableNameChecking -Global -ErrorAction Stop
        Write-Verbose "Reloaded module: $($_.Name)"
    } catch {
        Write-Verbose "Could not reload module $($_.Name): $($_.Exception.Message)"
    }
}
"#;

        // Step 1: Check system modules
        println!("{}", t!("Checking PowerShell modules..."));
        ctx.run_type()
            .execute(powershell)
            .args(["-NoProfile", "-Command", system_modules_update_script])
            .status_checked()?;

        // Step 2: Update regular modules
        println!("{}", t!("Updating regular modules..."));
        let mut update_args = vec!["-NoProfile", "-Command", regular_update_script];
        if ctx.config().verbose() {
            update_args.push("-Verbose");
        }

        ctx.run_type().execute(powershell).args(update_args).status_checked()?;

        // Step 3: Update system modules if needed
        println!("{}", t!("Handling system modules..."));
        ctx.run_type()
            .execute(powershell)
            .args(["-NoProfile", "-Command", system_update_script])
            .status_checked()?;

        // Step 4: Reload modules
        println!("{}", t!("Reloading modules..."));
        let mut reload_args = vec!["-NoProfile", "-Command", reload_script];
        if ctx.config().verbose() {
            reload_args.push("-Verbose");
        }

        ctx.run_type().execute(powershell).args(reload_args).status_checked()
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
