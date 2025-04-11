#[cfg(windows)]
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

    /// Helper function to clean translated strings by removing locale prefixes
    fn clean_translation(&self, text: impl Into<String>) -> String {
        let text = text.into();
        // Remove locale prefixes like "en-GB." from translated strings
        if let Some(idx) = text.find('.') {
            if text.chars().take(idx).all(|c| c.is_ascii_alphabetic() || c == '-') {
                return text[idx + 1..].to_string();
            }
        }
        text
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
            self.clean_translation(t!("Processing PowerShell modules...")),
            self.clean_translation(t!("Checking connectivity to PowerShell Gallery...")),
            self.clean_translation(t!("PowerShell Gallery is accessible")),
            self.clean_translation(t!(
                "PowerShell Gallery is not accessible. Module updates will be skipped."
            )),
            self.clean_translation(t!("Processing module: {moduleName}", moduleName = "$moduleName")),
            self.generate_module_unload_script(),
            self.clean_translation(t!("Updating module: {moduleName}", moduleName = "$moduleName")),
            update_command,
            self.clean_translation(t!(
                "Retry attempt {attempt} of {max}...",
                attempt = "$updateAttempts",
                max = "$maxAttempts"
            )),
            self.clean_translation(t!("Failed to update module after multiple attempts")),
            self.generate_module_reload_script(),
            self.clean_translation(t!(
                "Failed to process module: {moduleName} - {error}",
                moduleName = "$moduleName",
                error = "$($_.Exception.Message)"
            )),
            self.clean_translation(t!("Unable to connect to PowerShell Gallery. Module updates skipped.")),
            self.clean_translation(t!("Will still attempt to load existing modules")),
            self.clean_translation(t!("PowerShell module processing complete."))
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
            self.clean_translation(t!("Unloading module: {moduleName}", moduleName = "$moduleName"))
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
            self.clean_translation(t!("Reloading module: {moduleName}", moduleName = "$moduleName")),
            self.clean_translation(t!(
                "Successfully imported module: {moduleName}",
                moduleName = "$moduleName"
            )),
            self.clean_translation(t!(
                "Could not reload module: {moduleName} - {error}",
                moduleName = "$moduleName",
                error = "$($_.Exception.Message)"
            ))
        )
    }

    /// Creates a command to execute PowerShell with optional sudo elevation
    fn create_powershell_command(&self, ctx: &ExecutionContext) -> Result<Executor> {
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;

        let cmd = if let Some(sudo) = ctx.sudo() {
            let mut cmd = ctx.run_type().execute(sudo);
            // When using sudo, pass the PowerShell path as a single argument to prevent
            // the shell from splitting it, then pass default args separately
            cmd.arg(powershell);
            cmd
        } else {
            ctx.run_type().execute(powershell)
        };

        Ok(cmd)
    }

    /// Execute a PowerShell script with standard arguments
    fn execute_script(&self, ctx: &ExecutionContext, script: &str, print_elevation_message: bool) -> Result<()> {
        let mut cmd = self.create_powershell_command(ctx)?;

        // Check if this will be elevated and print message if requested
        let will_elevate = ctx.sudo().is_some();
        if will_elevate && print_elevation_message {
            println!(
                "{}",
                self.clean_translation(t!(
                    "This operation requires administrator privileges, expect a UAC prompt..."
                ))
            );
        }

        cmd.args(Self::default_args())
            .arg("-Command")
            .arg(script)
            .status_checked()
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("Powershell Modules Update"));
        let script = self.create_update_script(ctx);
        self.execute_script(ctx, &script, true)
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
            "Write-Output '{}'; Install-WindowsUpdate {} {}",
            powershell.clean_translation(t!("Starting Windows Update...")),
            if ctx.config().verbose() { "-Verbose" } else { "" },
            accept_all
        );

        // Pass false to avoid duplicate elevation message
        powershell.execute_script(ctx, &install_command, true)
    }

    pub fn microsoft_store(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("Microsoft Store"));
        println!("{}", t!("Scanning for updates..."));

        // Build the command with optional verbosity
        let verbose_flag = if ctx.config().verbose() { " -Verbose" } else { "" };

        // Create a PowerShell script that attempts only one method, with better feedback
        let ps_script = format!(
            r#"
            # This operation requires administrator privileges
            $isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
            if (-not $isAdmin) {{
                Write-Output "{}"
                # Don't exit - we'll still try to run the command and let it fail properly
            }}

            # Only attempt the primary MDM UpdateScanMethod - most reliable method
            try {{
                Write-Output "{}"
                $result = (Get-CimInstance{} -Namespace "Root\cimv2\mdm\dmmap" -ClassName "MDM_EnterpriseModernAppManagement_AppManagement01" -ErrorAction Stop | 
                Invoke-CimMethod{} -MethodName UpdateScanMethod -ErrorAction Stop).ReturnValue
                
                if ($result -eq 0) {{
                    Write-Output "SUCCESS_PRIMARY"
                }} else {{
                    Write-Output "FAIL_PRIMARY_NONZERO:$result"
                }}
            }} catch {{
                Write-Output "FAIL_PRIMARY_EXCEPTION:$($_.Exception.Message)"
            }}
            "#,
            powershell.clean_translation(t!(
                "Note: Administrator privileges required for Microsoft Store updates"
            )),
            powershell.clean_translation(t!("Attempting to update Microsoft Store apps using MDM method...")),
            verbose_flag,
            verbose_flag
        );

        // Execute the script with proper privilege handling - pass true since we need the message here
        match powershell.execute_script(ctx, &ps_script, true) {
            Ok(_) => {
                println!(
                    "{}",
                    t!("Success, Microsoft Store apps are being updated in the background")
                );
                Ok(())
            }
            Err(e) => {
                println!("{}: {}", t!("Microsoft Store update failed"), e);

                // Fall back to manual method - avoid re-printing separator
                println!("{}", t!("Attempting to open Microsoft Store updates page..."));
                let store_script = r#"$Launcher = [Windows.System.Launcher,Windows.System,ContentType=WindowsRuntime]; 
                    $Launcher::LaunchUriAsync([uri]'ms-windows-store://downloadsandupdates').GetAwaiter().GetResult()"#;

                // Don't print elevation message for fallbacks since we already showed one
                if let Err(e) = powershell.execute_script(ctx, store_script, false) {
                    println!("{}: {}", t!("Failed to open Microsoft Store"), e);
                } else {
                    println!(
                        "{}",
                        t!("Opened Microsoft Store updates page. Please check for updates manually.")
                    );
                }

                // Fall back to wsreset as last resort
                println!("{}", t!("Attempting to reset Microsoft Store..."));
                if let Err(e) = ctx.run_type().execute("wsreset.exe").arg("-i").status_checked() {
                    println!("{}: {}", t!("Failed to reset Microsoft Store"), e);
                } else {
                    println!(
                        "{}",
                        t!("Initiated Microsoft Store reset. Updates should begin shortly.")
                    );
                }

                Err(color_eyre::eyre::eyre!(
                    "Microsoft Store update failed. Administrator privileges may be required."
                ))
            }
        }
    }
}
