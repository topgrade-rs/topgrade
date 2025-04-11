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

    /// Creates the PowerShell script for updating modules
    fn create_update_script(&self, ctx: &ExecutionContext) -> String {
        let force_flag = self.get_force_flag(ctx);
        let update_command = self.build_update_command(force_flag, ctx.config().verbose());

        // Format the entire script using a template style for better readability
        format!(
            r#"Write-Host "{}" -ForegroundColor Cyan
# First test connectivity to PowerShell Gallery
$galleryAvailable = $false
Write-Host "{}" -ForegroundColor Cyan
try {{
  $request = [System.Net.WebRequest]::Create("https://www.powershellgallery.com/api/v2")
  $request.Method = "HEAD"
  $request.Timeout = 10000
  $response = $request.GetResponse()
  $galleryAvailable = $true
  $response.Close()
  Write-Host "{}" -ForegroundColor Green
}} catch {{
  Write-Host "{}" -ForegroundColor Red
  Write-Host "  $($_.Exception.Message)" -ForegroundColor Red
}}

if ($galleryAvailable) {{
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
        $updateAttempts = 0
        $maxAttempts = 2
        $updateSuccess = $false
        
        while (-not $updateSuccess -and $updateAttempts -lt $maxAttempts) {{
          try {{
            $updateAttempts++
            {}
            $updateSuccess = $true
          }} catch {{
            if ($updateAttempts -lt $maxAttempts) {{
              Write-Host "    {}" -ForegroundColor Yellow
              Start-Sleep -Seconds 2
            }} else {{
              Write-Host "    {}" -ForegroundColor Red
              Write-Host "    $($_.Exception.Message)" -ForegroundColor Red
            }}
          }}
        }}
        
        {}
      }}
    }} catch {{
      Write-Host "{}" -ForegroundColor Red
    }}
  }}
}} else {{
  Write-Host "{}" -ForegroundColor Red
  # Continue with module loading anyway, as they might still work
  Write-Host "{}" -ForegroundColor Yellow
}}
Write-Host "{}" -ForegroundColor Green"#,
            t!("Processing PowerShell modules..."),
            t!("Checking connectivity to PowerShell Gallery..."),
            t!("PowerShell Gallery is accessible"),
            t!("PowerShell Gallery is not accessible. Module updates will be skipped."),
            t!("Processing module: {moduleName}", moduleName = "$moduleName"),
            self.generate_module_unload_script(),
            t!("Updating module: {moduleName}", moduleName = "$moduleName"),
            update_command,
            t!(
                "Retry attempt {attempt} of {max}...",
                attempt = "$updateAttempts",
                max = "$maxAttempts"
            ),
            t!("Failed to update module after multiple attempts"),
            self.generate_module_reload_script(),
            t!(
                "Failed to process module: {moduleName} - {error}",
                moduleName = "$moduleName",
                error = "$($_.Exception.Message)"
            ),
            t!("Unable to connect to PowerShell Gallery. Module updates skipped."),
            t!("Will still attempt to load existing modules"),
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

    fn has_module(powershell: &PathBuf, module_name: &str) -> bool {
        Command::new(powershell)
            .args([
                "-NoProfile",
                "-Command",
                &format!("Get-Module -ListAvailable {}", module_name),
            ])
            .output_checked_utf8()
            .map(|result| !result.stdout.is_empty())
            .unwrap_or(false)
    }

    pub fn windows_update(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        debug_assert!(supports_windows_update(powershell));

        print_separator(t!("Windows Update"));

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

        // Use execute_script instead of run_ps_command to properly handle elevation
        powershell.execute_script(ctx, &install_command)
    }

    pub fn microsoft_store(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("Microsoft Store"));
        println!("{}", t!("Scanning for updates..."));

        // Get powershell path
        let powershell_path = require_option(powershell.path.as_ref(), t!("Powershell is not installed").to_string())?;

        // Build the command with optional verbosity
        let verbose_flag = if ctx.config().verbose() { " -Verbose" } else { "" };

        // Create a PowerShell script that tries each method sequentially using proper flow control
        let ps_script = format!(
            r#"
            $success = $false

            # Try primary method: MDM UpdateScanMethod
            try {{
                $result = (Get-CimInstance{} -Namespace "Root\cimv2\mdm\dmmap" -ClassName "MDM_EnterpriseModernAppManagement_AppManagement01" -ErrorAction Stop | 
                Invoke-CimMethod{} -MethodName UpdateScanMethod -ErrorAction Stop).ReturnValue
                if ($result -eq 0) {{
                    Write-Output "SUCCESS_PRIMARY"
                    $success = $true
                }} else {{
                    Write-Output "FAIL_PRIMARY_NONZERO:$result"
                }}
            }} catch {{
                Write-Output "FAIL_PRIMARY_EXCEPTION:$($_.Exception.Message)"
            }}

            # If primary method failed, try fallback method 1: Using WinRT API
            if (-not $success) {{
                try {{
                    $Launcher = [Windows.System.Launcher,Windows.System,ContentType=WindowsRuntime]
                    $result = $Launcher::LaunchUriAsync([uri]'ms-windows-store://downloadsandupdates').GetAwaiter().GetResult()
                    if ($result) {{
                        Write-Output "SUCCESS_FALLBACK1"
                        $success = $true
                    }} else {{
                        Write-Output "FAIL_FALLBACK1:LaunchUriAsync returned false"
                    }}
                }} catch {{
                    Write-Output "FAIL_FALLBACK1:$($_.Exception.Message)"
                }}
            }}

            # If both previous methods failed, try fallback method 2: WSReset command
            if (-not $success) {{
                try {{
                    Start-Process "wsreset.exe" -ArgumentList "-i"{} -ErrorAction Stop
                    Write-Output "SUCCESS_FALLBACK2"
                    $success = $true
                }} catch {{
                    Write-Output "FAIL_FALLBACK2:$($_.Exception.Message)"
                }}
            }}

            if (-not $success) {{
                Write-Output "FAIL_ALL_METHODS"
            }}
            "#,
            verbose_flag,
            verbose_flag,
            if ctx.config().verbose() { " -Verb RunAs" } else { "" }
        );

        // Execute the full script
        let output = match powershell.run_ps_command(powershell_path, &ps_script) {
            Ok(output) => output.trim().to_string(),
            Err(e) => {
                println!("{}: {}", t!("Error executing PowerShell command"), e);
                return Err(color_eyre::eyre::eyre!("Microsoft Store update failed: {}", e));
            }
        };

        // Process the output and provide appropriate feedback
        if output.contains("SUCCESS_PRIMARY") {
            println!(
                "{}",
                t!("Success, Microsoft Store apps are being updated in the background")
            );
            Ok(())
        } else if output.contains("SUCCESS_FALLBACK1") {
            println!(
                "{}",
                t!("Opened Microsoft Store updates page. Please check for updates manually.")
            );
            Ok(())
        } else if output.contains("SUCCESS_FALLBACK2") {
            println!(
                "{}",
                t!("Initiated Microsoft Store reset. Updates should begin shortly.")
            );
            Ok(())
        } else if output.contains("FAIL_PRIMARY_NONZERO:") {
            let code = output.split(':').nth(1).unwrap_or("unknown");
            println!(
                "{}",
                t!(
                    "Primary method failed with code: {code}. Trying alternative methods...",
                    code = code
                )
            );
            Err(color_eyre::eyre::eyre!(
                "Microsoft Store update failed with code {}",
                code
            ))
        } else if output.contains("FAIL_PRIMARY_EXCEPTION:") {
            let msg = output.split_once(':').map(|x| x.1).unwrap_or("unknown error");
            println!(
                "{}",
                t!(
                    "Primary method failed: {error}. Trying alternative methods...",
                    error = msg
                )
            );
            Err(color_eyre::eyre::eyre!("Microsoft Store update failed: {}", msg))
        } else {
            println!(
                "{}",
                t!("All Microsoft Store update methods failed. Please update manually.")
            );
            Err(color_eyre::eyre::eyre!("Microsoft Store update failed"))
        }
    }
}
