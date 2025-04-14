use std::cell::Cell;
use std::path::PathBuf;
use std::process::Command;
use std::borrow::Cow;

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

mod scripts {
    // PowerShell script templates
    pub(super) const GALLERY_CHECK_TEMPLATE: &str = r#"# First test connectivity to PowerShell Gallery
$galleryAvailable = $false
Write-Host "{checking_connectivity}" -ForegroundColor Cyan
try {{
  $request = [System.Net.WebRequest]::Create("https://www.powershellgallery.com/api/v2")
  $request.Method = "HEAD"
  $request.Timeout = 10000
  $response = $request.GetResponse()
  $galleryAvailable = $true
  $response.Close()
  Write-Host "{gallery_accessible}" -ForegroundColor Green
}} catch {{
  Write-Host "{gallery_not_accessible}" -ForegroundColor Red
  Write-Host "  $($_.Exception.Message)" -ForegroundColor Red
}}"#;

    pub(super) const MODULES_UPDATE_TEMPLATE: &str = r#"Get-Module -ListAvailable | Select-Object -Property Name -Unique | ForEach-Object {{
    $moduleName = $_.Name
    try {{
      # Only process modules installed via Install-Module
      if (Get-InstalledModule -Name $moduleName -ErrorAction SilentlyContinue) {{
        # Process each module individually - unload, update, reload
        Write-Host "{processing_module}" -ForegroundColor Cyan

        # Check if the module is loaded and unload it if necessary
        Write-Host "  {unloading_module}" -ForegroundColor Yellow
        if (Get-Module -Name $moduleName -ErrorAction SilentlyContinue) {{
          Remove-Module -Name $moduleName -Force -ErrorAction SilentlyContinue
        }} else {{
          Write-Host "    Module is not currently loaded" -ForegroundColor Yellow
        }}

        # Update the module
        Write-Host "  {updating_module}" -ForegroundColor Cyan
        $updateAttempts = 0
        $maxAttempts = 2
        $updateSuccess = $false

        while (-not $updateSuccess -and $updateAttempts -lt $maxAttempts) {{
          try {{
            $updateAttempts++
            {update_command}
            $updateSuccess = $true
          }} catch {{
            if ($updateAttempts -lt $maxAttempts) {{
              Write-Host "    {retry_attempt}" -ForegroundColor Yellow
              Start-Sleep -Seconds 2
            }} else {{
              Write-Host "    {update_failed}" -ForegroundColor Red
              Write-Host "    $($_.Exception.Message)" -ForegroundColor Red
            }}
          }}
        }}

        # Reload the module
        try {{
          Write-Host "  {reloading_module}" -ForegroundColor Green
          Import-Module $moduleName -ErrorAction Stop
          Write-Host "  {import_success}" -ForegroundColor Green
        }} catch {{
          Write-Host "  {import_failed}" -ForegroundColor Yellow
        }}
      }}
    }} catch {{
      Write-Host "{process_failed}" -ForegroundColor Red
    }}
  }}"#;

    #[cfg(windows)]
    pub(super) const MS_STORE_UPDATE_TEMPLATE: &str = r#"try {{
        Write-Output "{attempting_store_update}"
        $result = (Get-CimInstance{verbose_flag} -Namespace "Root\cimv2\mdm\dmmap" -ClassName "MDM_EnterpriseModernAppManagement_AppManagement01" -ErrorAction Stop |
        Invoke-CimMethod{verbose_flag} -MethodName UpdateScanMethod -ErrorAction Stop).ReturnValue

        if ($result -eq 0) {{
            Write-Output "{update_completed}"
        }} else {{
            Write-Output "FAIL_PRIMARY_NONZERO:$result"
        }}
    }} catch {{
        Write-Output "FAIL_PRIMARY_EXCEPTION:$($_.Exception.Message)"
    }}"#;
}

// Improved ScriptBuilder with more functional interface
#[derive(Clone)]
struct ScriptBuilder {
    translations: Vec<(String, String)>,
    template: String,
}

impl ScriptBuilder {
    fn new(template: &str) -> Self {
        Self {
            translations: Vec::new(),
            template: template.to_string(),
        }
    }

    // Return Self instead of &mut Self for better chaining
    fn add_translation(mut self, placeholder: &str, translation: impl Into<String>) -> Self {
        self.translations.push((placeholder.to_string(), translation.into()));
        self
    }

    fn with_param(mut self, placeholder: &str, value: &str) -> Self {
        self.translations.push((placeholder.to_string(), value.to_string()));
        self
    }

    // Build with automatic translation cleaning
    fn build(self, translator: impl Fn(&str) -> String) -> String {
        let mut result = self.template;
        for (placeholder, text) in self.translations {
            let translated = translator(&text);
            result = result.replace(&format!("{{{}}}", placeholder), &translated);
        }
        result
    }
}

pub struct Powershell {
    path: Option<PathBuf>,
    profile: Option<PathBuf>,
    uac_prompt_shown: Cell<bool>,
    #[cfg(windows)]
    windows_update_support: Cell<Option<bool>>,
}

impl Powershell {
    pub fn new() -> Self {
        let path = which("pwsh").or_else(|| which("powershell")).filter(|_| !is_dumb());
        let profile = Self::find_profile(&path);
        Powershell {
            path,
            profile,
            uac_prompt_shown: Cell::new(false),
            #[cfg(windows)]
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

    // Simplify translation with auto-cleaning
    fn t(&self, key: &str, params: Option<Vec<(&str, &str)>>) -> Cow<str> {
        let translated = if key.starts_with("en.") {
            // Direct translation key reference
            t!(key)
        } else {
            // Standard translation lookup
            match key {
                "Powershell is not installed" => t!("Powershell is not installed"),
                "Administrator privileges required - you will see a UAC prompt" => {
                    t!("Administrator privileges required - you will see a UAC prompt")
                }
                "Starting Windows Update..." => t!("Starting Windows Update..."),
                "Windows Update check completed" => t!("Windows Update check completed"),
                "Microsoft Store update failed" => t!("Microsoft Store update failed"),
                "Attempting to open Microsoft Store updates page..." => {
                    t!("Attempting to open Microsoft Store updates page...")
                }
                "Failed to open Microsoft Store" => t!("Failed to open Microsoft Store"),
                "Opened Microsoft Store updates page. Please check for updates manually." => {
                    t!("Opened Microsoft Store updates page. Please check for updates manually.")
                }
                "Attempting to reset Microsoft Store..." => t!("Attempting to reset Microsoft Store..."),
                "Failed to reset Microsoft Store" => t!("Failed to reset Microsoft Store"),
                "Initiated Microsoft Store reset. Updates should begin shortly." => {
                    t!("Initiated Microsoft Store reset. Updates should begin shortly.")
                }
                "Scanning for updates..." => t!("Scanning for updates..."),
                _ => Cow::Borrowed(key),
            }
        };

        if let Some(params) = params {
            // Handle parameter substitution properly
            let mut result = translated.to_string();
            for (key, value) in params {
                result = result.replace(&format!("{{{}}}", key), value);
            }
            self.clean_translation(&result).into()
        } else {
            let cleaned = self.clean_translation(&translated);
            if cleaned == translated {
                translated
            } else {
                Cow::Owned(cleaned)
            }
        }
    }

    // Helper function to clean translated strings by removing locale prefixes
    fn clean_translation(&self, text: &str) -> String {
        // Avoid allocating new string if no prefix exists
        match text.find('.') {
            Some(idx) if text.chars().take(idx).all(|c| c.is_ascii_alphabetic() || c == '-') => {
                text[idx + 1..].to_string()
            }
            _ => text.to_string(),
        }
    }

    /// Get flags for module update based on configuration
    fn get_update_flags(&self, ctx: &ExecutionContext) -> (String, String) {
        let force_flag = if ctx.config().yes(Step::Powershell) || ctx.config().powershell_force_modules_update() {
            " -Force"
        } else {
            ""
        };

        let verbose_flag = if ctx.config().verbose() { " -Verbose" } else { "" };

        (force_flag.to_string(), verbose_flag.to_string())
    }

    /// Creates the PowerShell script for updating modules
    fn create_update_script(&self, ctx: &ExecutionContext) -> String {
        let (force_flag, verbose_flag) = self.get_update_flags(ctx);
        let update_command = format!("Update-Module -Name $moduleName{}{}", verbose_flag, force_flag);

        // Create gallery check script using cleaner builder pattern
        let gallery_check_script = ScriptBuilder::new(scripts::GALLERY_CHECK_TEMPLATE)
            .add_translation(
                "checking_connectivity",
                "en.Checking connectivity to PowerShell Gallery...",
            )
            .add_translation("gallery_accessible", "en.PowerShell Gallery is accessible")
            .add_translation(
                "gallery_not_accessible",
                "en.PowerShell Gallery is not accessible. Module updates will be skipped.",
            )
            .build(|t| self.clean_translation(t));

        // Create modules update script with cleaner builder pattern
        let update_modules_script = ScriptBuilder::new(scripts::MODULES_UPDATE_TEMPLATE)
            .add_translation(
                "processing_module",
                "en.Processing module: {moduleName}",
            )
            .add_translation(
                "unloading_module",
                "en.Unloading module: {moduleName}",
            )
            .add_translation(
                "updating_module",
                "en.Updating module: {moduleName}",
            )
            .with_param("update_command", &update_command)
            .add_translation(
                "retry_attempt",
                "en.Retry attempt {attempt} of {max}...",
            )
            .add_translation("update_failed", "en.Failed to update module after multiple attempts")
            .add_translation(
                "reloading_module",
                "en.Reloading module: {moduleName}",
            )
            .add_translation(
                "import_success",
                "en.Successfully imported module: {moduleName}",
            )
            .add_translation(
                "import_failed",
                "en.Could not reload module: {moduleName} - {error}",
            )
            .add_translation(
                "process_failed",
                "en.Failed to process module: {moduleName} - {error}",
            )
            .build(|t| self.clean_translation(t));

        // Use direct t!() macro for these simple strings
        let processing_modules = t!("Processing PowerShell modules...");
        let gallery_connection_failed = t!("Unable to connect to PowerShell Gallery. Module updates skipped.");
        let will_load_modules = t!("Will still attempt to load existing modules");
        let processing_complete = t!("PowerShell module processing complete.");
        let update_check_completed = t!("PowerShell Modules update check completed");

        format!(
            r#"Write-Host "{}" -ForegroundColor Cyan

{gallery_check_script}

if ($galleryAvailable) {{
    {update_modules_script}
}} else {{
    Write-Host "{}" -ForegroundColor Red
    # Continue with module loading anyway, as they might still work
    Write-Host "{}" -ForegroundColor Yellow
}}

Write-Host "{}" -ForegroundColor Green
Write-Host "{}" -ForegroundColor Green"#,
            self.clean_translation(&processing_modules),
            self.clean_translation(&gallery_connection_failed),
            self.clean_translation(&will_load_modules),
            self.clean_translation(&processing_complete),
            self.clean_translation(&update_check_completed)
        )
    }

    // Consolidated command creation with improved error handling
    fn create_command(&self, ctx: &ExecutionContext) -> Result<Executor> {
        let powershell = require_option(self.path.as_ref(), self.t("Powershell is not installed", None))?;

        Ok(if let Some(sudo) = ctx.sudo() {
            let mut cmd = ctx.run_type().execute(sudo);
            cmd.arg(powershell);
            cmd
        } else {
            ctx.run_type().execute(powershell)
        })
    }

    // Execute script with improved error handling
    fn execute_script(&self, ctx: &ExecutionContext, script: &str) -> Result<()> {
        // Check elevation status first
        self.check_elevation(ctx);

        // Check execution policy on Windows
        #[cfg(windows)]
        self.execution_policy_args_if_needed()?;

        // Create and execute command in one go
        self.create_command(ctx)?
            .args(Self::default_args())
            .arg(PS_COMMAND)
            .arg(script)
            .status_checked()
    }

    // Extract elevation check to separate method to improve readability
    fn check_elevation(&self, ctx: &ExecutionContext) {
        if ctx.sudo().is_some() && !Self::is_process_elevated() && !self.uac_prompt_shown.get() {
            println!(
                "{}",
                self.clean_translation(&t!("Administrator privileges required - you will see a UAC prompt"))
            );
            self.uac_prompt_shown.set(true);
        }
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
            println!("{}", self.clean_translation(&t!("Scanning for updates...")));
        }

        // Execute the operation
        let result = operation();

        // Show completion message if operation succeeded and we're not elevating
        if result.is_ok() && !will_elevate {
            // Use string literal with parameter substitution
            let completed_message = match operation_name {
                "PowerShell Modules" => t!("PowerShell Modules update check completed"),
                "Windows Update" => t!("Windows Update check completed"),
                "Microsoft Store" => t!("Microsoft Store update check completed"),
                _ => t!("{operation_name} check completed", operation_name = operation_name),
            };
            println!("{}", self.clean_translation(&completed_message));
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
        // Check if Windows Update is supported before attempting to run it
        if !self.supports_windows_update() {
            return Err(color_eyre::eyre::eyre!(
                "PSWindowsUpdate module is not installed. Windows Update is not available."
            ));
        }

        let verbose_flag = if ctx.config().verbose() { " -Verbose" } else { "" };
        let accept_flag = if ctx.config().accept_all_windows_updates() {
            " -AcceptAll"
        } else {
            ""
        };

        // Build command with appropriate flags
        let script = format!(
            "Write-Output '{}'; Install-WindowsUpdate{}{} -Confirm:$false; Write-Output '{}'",
            self.clean_translation(&t!("Starting Windows Update...")),
            verbose_flag,
            accept_flag,
            self.clean_translation(&t!("Windows Update check completed"))
        );

        self.execute_operation(ctx, "Windows Update", || self.execute_script(ctx, &script))
    }

    // Improved Windows Store script creation with better error handling
    #[cfg(windows)]
    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        windows::microsoft_store(self, ctx)
    }

    /// Checks if PowerShell execution policy is properly set
    pub fn execution_policy_args_if_needed(&self) -> Result<()> {
        if !self.is_execution_policy_set("RemoteSigned") {
            Err(color_eyre::eyre::eyre!(
                "PowerShell execution policy is too restrictive. \
                Please run 'Set-ExecutionPolicy RemoteSigned -Scope CurrentUser' in PowerShell \
                (or use Unrestricted/Bypass if you're sure about the security implications)"
            ))
        } else {
            Ok(())
        }
    }

    /// Checks if the current execution policy is at least as permissive as the required policy
    fn is_execution_policy_set(&self, policy: &str) -> bool {
        if let Some(powershell) = &self.path {
            // These policies are ordered from most restrictive to least restrictive
            let valid_policies = ["Restricted", "AllSigned", "RemoteSigned", "Unrestricted", "Bypass"];

            // Find the index of our target policy
            let target_idx = valid_policies.iter().position(|&p| p == policy);

            let output = Command::new(powershell)
                .args([PS_NO_PROFILE, PS_COMMAND, "Get-ExecutionPolicy"])
                .output_checked_utf8();

            if let Ok(output) = output {
                let current_policy = output.stdout.trim();

                // Find the index of the current policy
                let current_idx = valid_policies.iter().position(|&p| p == current_policy);

                // Check if current policy exists and is at least as permissive as the target
                return match (current_idx, target_idx) {
                    (Some(current), Some(target)) => current >= target,
                    _ => false,
                };
            }
        }
        false
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
                    "if (Get-Module -ListAvailable {} -ErrorAction SilentlyContinue) {{ Write-Output 'true' }}",
                    module_name
                ),
            ])
            .output_checked_utf8()
            .map(|result| result.stdout.trim() == "true")
            .unwrap_or(false)
    }

    // More modular approach to Microsoft Store updates
    pub fn microsoft_store(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        let verbose_flag = if ctx.config().verbose() { " -Verbose" } else { "" };
        let ps_script = create_microsoft_store_script(powershell, verbose_flag);

        powershell.execute_operation(ctx, "Microsoft Store", || {
            match powershell.execute_script(ctx, &ps_script) {
                Ok(_) => Ok(()),
                Err(e) => {
                    // Log error and try fallbacks
                    println!(
                        "{}: {}",
                        powershell.clean_translation(&t!("Microsoft Store update failed")),
                        e
                    );

                    try_microsoft_store_fallbacks(powershell, ctx)?;

                    // Still return an error for proper status tracking
                    Err(color_eyre::eyre::eyre!(
                        "Primary Microsoft Store update method failed. Fallbacks attempted."
                    ))
                }
            }
        })
    }

    fn create_microsoft_store_script(powershell: &Powershell, verbose_flag: &str) -> String {
        ScriptBuilder::new(scripts::MS_STORE_UPDATE_TEMPLATE)
            .add_translation(
                "attempting_store_update",
                "en.Attempting to update Microsoft Store apps using MDM method...",
            )
            .with_param("verbose_flag", verbose_flag)
            .add_translation("update_completed", "en.Microsoft Store update check completed")
            .build(|t| powershell.clean_translation(t))
    }

    fn try_microsoft_store_fallbacks(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        // Prepare translations to avoid repeated calls
        let translations = [
            t!("Attempting to open Microsoft Store updates page..."),
            t!("Failed to open Microsoft Store"),
            t!("Opened Microsoft Store updates page. Please check for updates manually."),
            t!("Attempting to reset Microsoft Store..."),
            t!("Failed to reset Microsoft Store"),
            t!("Initiated Microsoft Store reset. Updates should begin shortly."),
        ]
        .map(|msg| powershell.clean_translation(&msg).to_string());

        // First fallback: open Microsoft Store updates page
        println!("{}", translations[0]);

        // Try to open the Microsoft Store updates page
        let store_script = r#"$Launcher = [Windows.System.Launcher,Windows.System,ContentType=WindowsRuntime];
                $Launcher::LaunchUriAsync([uri]'ms-windows-store://downloadsandupdates').GetAwaiter().GetResult()"#;

        if let Err(e) = powershell.execute_script(ctx, store_script) {
            println!("{}: {}", translations[1], e);
        } else {
            println!("{}", translations[2]);
        }

        // Second fallback: wsreset
        println!("{}", translations[3]);

        if let Err(e) = ctx.run_type().execute("wsreset.exe").arg("-i").status_checked() {
            println!("{}: {}", translations[4], e);
        } else {
            println!("{}", translations[5]);
        }

        Ok(())
    }
}

