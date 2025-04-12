use std::cell::Cell;
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

// String constants for common PowerShell arguments
const PS_NO_PROFILE: &str = "-NoProfile";
const PS_NO_LOGO: &str = "-NoLogo";
const PS_NON_INTERACTIVE: &str = "-NonInteractive";
const PS_COMMAND: &str = "-Command";

pub struct Powershell {
    path: Option<PathBuf>,
    profile: Option<PathBuf>,
    uac_prompt_shown: Cell<bool>,
    windows_update_support: Cell<Option<bool>>,
}

impl Powershell {
    /// Returns a powershell instance.
    ///
    /// If the powershell binary is not found, or the current terminal is dumb
    /// then the instance of this struct will skip all the powershell steps.
    pub fn new() -> Self {
        let path = which("pwsh").or_else(|| which("powershell")).filter(|_| !is_dumb());
        let profile = Self::find_profile(&path);
        Powershell {
            path,
            profile,
            uac_prompt_shown: Cell::new(false),
            windows_update_support: Cell::new(None),
        }
    }

    /// Helper to find the PowerShell profile path
    fn find_profile(path: &Option<PathBuf>) -> Option<PathBuf> {
        path.as_ref().and_then(|path| {
            Command::new(path)
                .args(Self::default_args())
                .arg(PS_COMMAND)
                .arg("Split-Path $profile")
                .output_checked_utf8()
                .map(|output| PathBuf::from(output.stdout.trim()))
                .and_then(super::super::utils::PathExt::require)
                .ok()
        })
    }

    /// Returns the default PowerShell command arguments used in most commands
    fn default_args() -> [&'static str; 3] {
        [PS_NO_PROFILE, PS_NO_LOGO, PS_NON_INTERACTIVE]
    }

    #[cfg(windows)]
    pub fn windows_powershell() -> Self {
        Powershell {
            path: which("powershell").filter(|_| !is_dumb()),
            profile: None,
            uac_prompt_shown: Cell::new(false),
            windows_update_support: Cell::new(None),
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

        // Assemble script sections
        let script_header = self.generate_script_header();
        let gallery_check = self.generate_gallery_check();
        let module_processing = self.generate_module_processing(&update_command);
        let script_footer = self.generate_script_footer();

        format!(
            "{}\n{}\n{}\n{}",
            script_header, gallery_check, module_processing, script_footer
        )
    }

    /// Generate the script header
    fn generate_script_header(&self) -> String {
        format!(
            r#"Write-Host "{}" -ForegroundColor Cyan"#,
            self.clean_translation(t!("Processing PowerShell modules..."))
        )
    }

    /// Generate the gallery connectivity check section
    fn generate_gallery_check(&self) -> String {
        format!(
            r#"# First test connectivity to PowerShell Gallery
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
}}"#,
            self.clean_translation(t!("Checking connectivity to PowerShell Gallery...")),
            self.clean_translation(t!("PowerShell Gallery is accessible")),
            self.clean_translation(t!(
                "PowerShell Gallery is not accessible. Module updates will be skipped."
            ))
        )
    }

    /// Generate the module processing section
    fn generate_module_processing(&self, update_command: &str) -> String {
        format!(
            r#"
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
}}"#,
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
            self.clean_translation(t!("Will still attempt to load existing modules"))
        )
    }

    /// Generate script footer
    fn generate_script_footer(&self) -> String {
        format!(
            r#"Write-Host "{}" -ForegroundColor Green"#,
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
    fn execute_script(&self, ctx: &ExecutionContext, script: &str) -> Result<()> {
        let mut cmd = self.create_powershell_command(ctx)?;

        // Check if this will be elevated and update prompt flag BEFORE executing
        let will_elevate = ctx.sudo().is_some();

        // Only show UAC prompt message if we will need to elevate AND we haven't shown the message yet
        // AND we're not already running as admin
        if will_elevate && !self.uac_prompt_shown.get() && !Self::is_process_elevated() {
            // Show UAC prompt message once before any elevation occurs
            println!(
                "{}",
                self.clean_translation(t!("Administrator privileges required - you will see a UAC prompt"))
            );
            // Mark as shown for the instance using Cell
            self.uac_prompt_shown.set(true);
        }

        cmd.args(Self::default_args())
            .arg(PS_COMMAND)
            .arg(script)
            .status_checked()
    }

    // Helper function to detect if current process is already elevated
    #[cfg(windows)]
    fn is_process_elevated() -> bool {
        use std::process::Command;

        // Use PowerShell to check if the current process is elevated
        // This command returns "True" if running as admin, "False" otherwise
        match Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "[bool](([System.Security.Principal.WindowsIdentity]::GetCurrent()).groups -match 'S-1-5-32-544')",
            ])
            .output_checked_utf8()
        {
            Ok(output) => {
                let result = output.stdout.trim().to_string();
                result.eq_ignore_ascii_case("true")
            }
            Err(_) => false,
        }
    }

    #[cfg(not(windows))]
    fn is_process_elevated() -> bool {
        false // On non-Windows platforms, we don't need this check
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("Powershell Modules Update"));

        // Only show scanning message if no UAC prompt will be shown
        if !self.will_show_uac_prompt(ctx) {
            println!("{}", self.clean_translation(t!("Scanning for updates...")));
        }

        // No need for duplicate UAC message here - execute_script will handle it
        let script = self.create_update_script(ctx);
        let result = self.execute_script(ctx, &script);

        // Add success message if script executed successfully AND elevation occurred
        if result.is_ok() {
            if self.did_elevation_occur(ctx) {
                println!(
                    "{}",
                    self.clean_translation(t!("PowerShell update check completed successfully"))
                );
            } else {
                println!(
                    "{}",
                    self.clean_translation(t!("PowerShell Modules update check completed"))
                );
            }
        }

        result
    }

    /// Helper to determine if a UAC prompt will be shown for this operation
    fn will_show_uac_prompt(&self, ctx: &ExecutionContext) -> bool {
        let will_elevate = ctx.sudo().is_some();
        will_elevate && !self.uac_prompt_shown.get() && !Self::is_process_elevated()
    }

    /// Helper to check if elevation actually occurred during an operation
    fn did_elevation_occur(&self, ctx: &ExecutionContext) -> bool {
        // Elevation occurred if sudo was specified AND either:
        // 1. We showed a UAC prompt, or
        // 2. Process was already elevated (no prompt needed but still elevated)
        ctx.sudo().is_some() && (self.uac_prompt_shown.get() || Self::is_process_elevated())
    }
}

#[cfg(windows)]
impl Powershell {
    pub fn supports_windows_update(&self) -> bool {
        // Check cached result first
        if let Some(supports) = self.windows_update_support.get() {
            return supports;
        }

        // If no cached result, perform the check
        let result = self
            .path
            .as_ref()
            .map(|p| windows::has_module(p, "PSWindowsUpdate"))
            .unwrap_or(false);

        // Cache the result for future calls
        self.windows_update_support.set(Some(result));

        result
    }

    pub fn windows_update(&self, ctx: &ExecutionContext) -> Result<()> {
        // Remove the debug_assert since we're now using the cached check
        // and this would be redundant

        let accept_all = if ctx.config().accept_all_windows_updates() {
            "-AcceptAll"
        } else {
            ""
        };

        // Only show scanning message if no UAC prompt will be shown
        if !self.will_show_uac_prompt(ctx) {
            println!("{}", self.clean_translation(t!("Scanning for updates...")));
        }

        // No need for a separate UAC prompt message - execute_script will handle it
        let install_command = format!(
            "Write-Output '{}'; Install-WindowsUpdate {} {}",
            self.clean_translation(t!("Starting Windows Update...")),
            if ctx.config().verbose() { "-Verbose" } else { "" },
            accept_all
        );

        // Use execute_script to properly handle elevation
        match self.execute_script(ctx, &install_command) {
            Ok(_) => {
                // Provide a more accurate status message
                if self.did_elevation_occur(ctx) {
                    println!(
                        "{}",
                        self.clean_translation(t!("Windows Update check completed successfully"))
                    );
                } else {
                    println!("{}", self.clean_translation(t!("Windows Update check completed")));
                }
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        windows::microsoft_store(self, ctx)
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    // Removed the unused function windows_update

    pub fn has_module(powershell: &PathBuf, module_name: &str) -> bool {
        // Use -ErrorAction SilentlyContinue to avoid prompts and error messages
        Command::new(powershell)
            .args([
                PS_NO_PROFILE,
                PS_NO_LOGO, // Added to be consistent with other commands
                PS_COMMAND,
                &format!(
                    "Get-Module -ListAvailable {} -ErrorAction SilentlyContinue",
                    module_name
                ),
            ])
            .output_checked_utf8()
            .map(|result| !result.stdout.is_empty())
            .unwrap_or(false)
    }

    pub fn microsoft_store(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        // Only show scanning message if no UAC prompt will be shown
        if !powershell.will_show_uac_prompt(ctx) {
            println!("{}", powershell.clean_translation(t!("Scanning for updates...")));
        }

        // Build the command with optional verbosity
        let verbose_flag = if ctx.config().verbose() { " -Verbose" } else { "" };

        // Comment out admin message in PowerShell output - our execute_script will handle it
        let ps_script = format!(
            r#"
            # This operation requires administrator privileges
            $isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
            if (-not $isAdmin) {{
                # Skip admin message - Topgrade handles this
                # Write-Output "{}"
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
            powershell.clean_translation(t!("Administrator privileges required for Microsoft Store updates")),
            powershell.clean_translation(t!("Attempting to update Microsoft Store apps using MDM method...")),
            verbose_flag,
            verbose_flag
        );

        // Execute the script and handle potential fallbacks
        handle_microsoft_store_update(powershell, ctx, ps_script)
    }

    // Helper function to handle Microsoft Store update with fallbacks
    fn handle_microsoft_store_update(powershell: &Powershell, ctx: &ExecutionContext, ps_script: String) -> Result<()> {
        match powershell.execute_script(ctx, &ps_script) {
            Ok(_) => {
                // Provide a more accurate status message
                if powershell.did_elevation_occur(ctx) {
                    println!(
                        "{}",
                        powershell.clean_translation(t!("Microsoft Store update check completed successfully"))
                    );
                } else {
                    println!(
                        "{}",
                        powershell.clean_translation(t!("Microsoft Store update check completed"))
                    );
                }
                Ok(())
            }
            Err(e) => {
                println!(
                    "{}: {}",
                    powershell.clean_translation(t!("Microsoft Store update failed")),
                    e
                );

                // Try fallback methods
                try_microsoft_store_fallbacks(powershell, ctx)?;

                Err(color_eyre::eyre::eyre!(
                    "Microsoft Store update failed. Administrator privileges may be required."
                ))
            }
        }
    }

    // Helper for fallback methods
    fn try_microsoft_store_fallbacks(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        // First fallback: open Microsoft Store updates page
        println!(
            "{}",
            powershell.clean_translation(t!("Attempting to open Microsoft Store updates page..."))
        );
        let store_script = r#"$Launcher = [Windows.System.Launcher,Windows.System,ContentType=WindowsRuntime];
                $Launcher::LaunchUriAsync([uri]'ms-windows-store://downloadsandupdates').GetAwaiter().GetResult()"#;

        if let Err(e) = powershell.execute_script(ctx, store_script) {
            println!(
                "{}: {}",
                powershell.clean_translation(t!("Failed to open Microsoft Store")),
                e
            );
        } else {
            println!(
                "{}",
                powershell.clean_translation(t!(
                    "Opened Microsoft Store updates page. Please check for updates manually."
                ))
            );
        }

        // Second fallback: wsreset
        println!(
            "{}",
            powershell.clean_translation(t!("Attempting to reset Microsoft Store..."))
        );
        if let Err(e) = ctx.run_type().execute("wsreset.exe").arg("-i").status_checked() {
            println!(
                "{}: {}",
                powershell.clean_translation(t!("Failed to reset Microsoft Store")),
                e
            );
        } else {
            println!(
                "{}",
                powershell.clean_translation(t!("Initiated Microsoft Store reset. Updates should begin shortly."))
            );
        }

        Ok(())
    }
}
