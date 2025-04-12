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
        let force_flag = if ctx.config().yes(Step::Powershell) || ctx.config().powershell_force_modules_update() {
            " -Force"
        } else {
            ""
        };

        let verbose_flag = if ctx.config().verbose() { " -Verbose" } else { "" };
        let update_command = format!("Update-Module -Name $moduleName{}{}", verbose_flag, force_flag);

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

        # Check if the module is loaded and unload it if necessary
        Write-Host "  {}" -ForegroundColor Yellow
        if (Get-Module -Name $moduleName -ErrorAction SilentlyContinue) {{
          Remove-Module -Name $moduleName -Force -ErrorAction SilentlyContinue
        }} else {{
          Write-Host "    Module is not currently loaded" -ForegroundColor Yellow
        }}

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
}} else {{
  Write-Host "{}" -ForegroundColor Red
  # Continue with module loading anyway, as they might still work
  Write-Host "{}" -ForegroundColor Yellow
}}

Write-Host "{}" -ForegroundColor Green
Write-Host "{}" -ForegroundColor Green"#,
            self.clean_translation(t!("Processing PowerShell modules...")),
            self.clean_translation(t!("Checking connectivity to PowerShell Gallery...")),
            self.clean_translation(t!("PowerShell Gallery is accessible")),
            self.clean_translation(t!(
                "PowerShell Gallery is not accessible. Module updates will be skipped."
            )),
            self.clean_translation(t!("Processing module: {moduleName}", moduleName = "$moduleName")),
            self.clean_translation(t!("Unloading module: {moduleName}", moduleName = "$moduleName")),
            self.clean_translation(t!("Updating module: {moduleName}", moduleName = "$moduleName")),
            update_command,
            self.clean_translation(t!(
                "Retry attempt {attempt} of {max}...",
                attempt = "$updateAttempts",
                max = "$maxAttempts"
            )),
            self.clean_translation(t!("Failed to update module after multiple attempts")),
            self.clean_translation(t!("Reloading module: {moduleName}", moduleName = "$moduleName")),
            self.clean_translation(t!(
                "Successfully imported module: {moduleName}",
                moduleName = "$moduleName"
            )),
            self.clean_translation(t!(
                "Could not reload module: {moduleName} - {error}",
                moduleName = "$moduleName",
                error = "$($_.Exception.Message)"
            )),
            self.clean_translation(t!(
                "Failed to process module: {moduleName} - {error}",
                moduleName = "$moduleName",
                error = "$($_.Exception.Message)"
            )),
            self.clean_translation(t!("Unable to connect to PowerShell Gallery. Module updates skipped.")),
            self.clean_translation(t!("Will still attempt to load existing modules")),
            self.clean_translation(t!("PowerShell module processing complete.")),
            self.clean_translation(t!("PowerShell Modules update check completed"))
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

    /// Handle UAC prompts and execute a PowerShell script with standard arguments
    fn execute_script(&self, ctx: &ExecutionContext, script: &str) -> Result<()> {
        // Check elevation status before creating command to avoid resource allocation if unnecessary
        if ctx.sudo().is_some() && !Self::is_process_elevated() && !self.uac_prompt_shown.get() {
            println!(
                "{}",
                self.clean_translation(t!("Administrator privileges required - you will see a UAC prompt"))
            );
            self.uac_prompt_shown.set(true);
        }

        // Create and execute the command
        self.create_powershell_command(ctx)?
            .args(Self::default_args())
            .arg(PS_COMMAND)
            .arg(script)
            .status_checked()
    }

    // Helper function to detect if current process is already elevated
    #[cfg(windows)]
    fn is_process_elevated() -> bool {
        use std::process::Command;

        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "[bool](([System.Security.Principal.WindowsIdentity]::GetCurrent()).groups -match 'S-1-5-32-544')",
            ])
            .output_checked_utf8()
            .map(|output| output.stdout.trim().to_lowercase() == "true")
            .unwrap_or(false)
    }

    #[cfg(not(windows))]
    fn is_process_elevated() -> bool {
        false // On non-Windows platforms, we don't need this check
    }

    /// Execute an operation with standard messaging and UAC handling
    fn execute_operation<F>(&self, ctx: &ExecutionContext, operation_name: &str, operation: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        let will_elevate = ctx.sudo().is_some() && !self.uac_prompt_shown.get() && !Self::is_process_elevated();

        // Only show scanning message if no UAC prompt will be shown
        if !will_elevate {
            println!("{}", self.clean_translation(t!("Scanning for updates...")));
        }

        // Execute the operation
        let result = operation();

        // Show completion message if operation succeeded and we're not elevating
        if result.is_ok() && !will_elevate {
            println!(
                "{}",
                self.clean_translation(t!("{operation_name} check completed", operation_name = operation_name))
            );
        }

        result
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("Powershell Modules Update"));

        self.execute_operation(ctx, "PowerShell Modules", || {
            let script = self.create_update_script(ctx);
            self.execute_script(ctx, &script)
        })
    }
}

#[cfg(windows)]
impl Powershell {
    pub fn supports_windows_update(&self) -> bool {
        // Use cached result if available
        if let Some(supports) = self.windows_update_support.get() {
            return supports;
        }

        // Check if the PSWindowsUpdate module is available
        let result = self
            .path
            .as_ref()
            .is_some_and(|p| windows::has_module(p, "PSWindowsUpdate"));

        // Cache the result for future calls
        self.windows_update_support.set(Some(result));

        result
    }

    pub fn windows_update(&self, ctx: &ExecutionContext) -> Result<()> {
        // Build command with appropriate flags
        let script = format!(
            "Write-Output '{}'; Install-WindowsUpdate{}{} -Confirm:$false; Write-Output '{}'",
            self.clean_translation(t!("Starting Windows Update...")),
            if ctx.config().verbose() { " -Verbose" } else { "" },
            if ctx.config().accept_all_windows_updates() {
                " -AcceptAll"
            } else {
                ""
            },
            self.clean_translation(t!("Windows Update check completed"))
        );

        self.execute_operation(ctx, "Windows Update", || self.execute_script(ctx, &script))
    }

    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        windows::microsoft_store(self, ctx)
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    pub fn has_module(powershell: &PathBuf, module_name: &str) -> bool {
        Command::new(powershell)
            .args([
                PS_NO_PROFILE,
                PS_NO_LOGO,
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
        let verbose_flag = if ctx.config().verbose() { " -Verbose" } else { "" };

        let ps_script = format!(
            r#"try {{
                Write-Output "{}"
                $result = (Get-CimInstance{} -Namespace "Root\cimv2\mdm\dmmap" -ClassName "MDM_EnterpriseModernAppManagement_AppManagement01" -ErrorAction Stop |
                Invoke-CimMethod{} -MethodName UpdateScanMethod -ErrorAction Stop).ReturnValue

                if ($result -eq 0) {{
                    Write-Output "{}"
                }} else {{
                    Write-Output "FAIL_PRIMARY_NONZERO:$result"
                }}
            }} catch {{
                Write-Output "FAIL_PRIMARY_EXCEPTION:$($_.Exception.Message)"
            }}"#,
            powershell.clean_translation(t!("Attempting to update Microsoft Store apps using MDM method...")),
            verbose_flag,
            verbose_flag,
            powershell.clean_translation(t!("Microsoft Store update check completed"))
        );

        powershell.execute_operation(ctx, "Microsoft Store", || {
            if let Err(e) = powershell.execute_script(ctx, &ps_script) {
                println!(
                    "{}: {}",
                    powershell.clean_translation(t!("Microsoft Store update failed")),
                    e
                );

                // If primary method fails, try fallbacks
                try_microsoft_store_fallbacks(powershell, ctx)?;

                // Still return an error to indicate primary method failed
                return Err(color_eyre::eyre::eyre!(
                    "Microsoft Store update failed. Administrator privileges may be required."
                ));
            }
            Ok(())
        })
    }

    fn try_microsoft_store_fallbacks(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        // First fallback: open Microsoft Store updates page
        println!(
            "{}",
            powershell.clean_translation(t!("Attempting to open Microsoft Store updates page..."))
        );

        // Try to open the Microsoft Store updates page
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
