use std::cell::Cell;
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

mod scripts {
    // PowerShell script templates
    pub(super) const GALLERY_CHECK_TEMPLATE: &str = r#"# First test connectivity to PowerShell Gallery
$galleryAvailable = $false
Write-Host "{checking_connectivity}" -ForegroundColor Cyan

# Try multiple methods to check connectivity
try {
    # Method 1: Direct Find-Module test
    try {
        Write-Host "  Trying direct Find-Module test..." -ForegroundColor Yellow
        $module = Find-Module -Name PowerShellGet -Repository PSGallery -ErrorAction Stop
        if ($module) {
            $galleryAvailable = $true
            Write-Host "{gallery_accessible}" -ForegroundColor Green
        }
    }
    catch {
        Write-Host "  Direct Find-Module failed: $($_.Exception.Message)" -ForegroundColor Yellow
        
        # Method 2: Check if PSGallery is registered
        try {
            Write-Host "  Checking PSGallery registration..." -ForegroundColor Yellow
            $repository = Get-PSRepository -Name PSGallery -ErrorAction Stop
            
            if ($repository.InstallationPolicy -eq 'Trusted') {
                $galleryAvailable = $true
                Write-Host "{gallery_accessible}" -ForegroundColor Green
            }
            else {
                # Method 3: Use a background job with longer timeout
                Write-Host "  Using background job with 10-second timeout..." -ForegroundColor Yellow
                $moduleSearchJob = Start-Job -ScriptBlock { 
                    Find-Module -Name PowerShellGet -Repository PSGallery -ErrorAction Stop | Select-Object -First 1
                }
                
                if (Wait-Job $moduleSearchJob -Timeout 10) {
                    $result = Receive-Job $moduleSearchJob
                    if ($result) {
                        $galleryAvailable = $true
                        Write-Host "{gallery_accessible}" -ForegroundColor Green
                    }
                }
                
                # Ensure the job is cleaned up
                Remove-Job $moduleSearchJob -Force -ErrorAction SilentlyContinue
            }
        }
        catch {
            Write-Host "  PSRepository check failed: $($_.Exception.Message)" -ForegroundColor Yellow
            
            # Method 4: Direct web request with longer timeout
            try {
                Write-Host "  Trying direct web request..." -ForegroundColor Yellow
                $request = [System.Net.WebRequest]::Create("https://www.powershellgallery.com/api/v2")
                $request.Method = "HEAD"
                $request.Timeout = 10000
                $response = $request.GetResponse()
                $galleryAvailable = $true
                $response.Close()
                Write-Host "{gallery_accessible}" -ForegroundColor Green
            }
            catch {
                Write-Host "  Web request failed: $($_.Exception.Message)" -ForegroundColor Yellow
            }
        }
    }
}
catch {
    Write-Host "  All connectivity checks failed" -ForegroundColor Red
}

# Final result
if ($galleryAvailable) {
    Write-Host "Gallery connectivity confirmed!" -ForegroundColor Green
} else {
    Write-Host "{gallery_not_accessible}" -ForegroundColor Red
    Write-Host "  Note: If you can use Find-Module manually, there may be an issue with the detection logic." -ForegroundColor Yellow
}"#;

    pub(super) const MODULES_UPDATE_TEMPLATE: &str = r#"Get-Module -ListAvailable | Select-Object -Property Name -Unique | ForEach-Object {
    $moduleName = $_.Name
    try {
      # Only process modules installed via Install-Module
      if (Get-InstalledModule -Name $moduleName -ErrorAction SilentlyContinue) {
        # Process each module individually - unload, update, reload
        Write-Host ("Processing module: {0}" -f $moduleName) -ForegroundColor Cyan

        # Check if the module is loaded and unload it if necessary
        Write-Host ("  Unloading module: {0}" -f $moduleName) -ForegroundColor Yellow
        if (Get-Module -Name $moduleName -ErrorAction SilentlyContinue) {
          Remove-Module -Name $moduleName -Force -ErrorAction SilentlyContinue
        } else {
          Write-Host "    Module is not currently loaded" -ForegroundColor Yellow
        }

        # Update the module
        Write-Host ("  Updating module: {0}" -f $moduleName) -ForegroundColor Cyan
        $updateAttempts = 0
        $maxAttempts = 2
        $updateSuccess = $false

        while (-not $updateSuccess -and $updateAttempts -lt $maxAttempts) {
          try {
            $updateAttempts++
            {update_command}
            $updateSuccess = $true
          } catch {
            if ($updateAttempts -lt $maxAttempts) {
              Write-Host ("    Retry attempt {0} of {1}..." -f $updateAttempts, $maxAttempts) -ForegroundColor Yellow
              Start-Sleep -Seconds 2
            } else {
              Write-Host "    Failed to update module after multiple attempts" -ForegroundColor Red
              Write-Host ("    " + $_.Exception.Message) -ForegroundColor Red
            }
          }
        }

        # Reload the module
        try {
          Write-Host ("  Reloading module: {0}" -f $moduleName) -ForegroundColor Green
          Import-Module $moduleName -ErrorAction Stop
          Write-Host ("  Successfully imported module: {0}" -f $moduleName) -ForegroundColor Green
        } catch {
          Write-Host ("  Could not reload module: {0}" -f $moduleName) -ForegroundColor Yellow
          Write-Host ("    " + $_.Exception.Message) -ForegroundColor Yellow
        }
      }
    } catch {
      Write-Host ("Failed to process module: {0}" -f $moduleName) -ForegroundColor Red
      Write-Host ("    " + $_.Exception.Message) -ForegroundColor Red
    }
  }"#;

    #[cfg(windows)]
    pub(super) const MS_STORE_UPDATE_TEMPLATE: &str = r#"try {
        Write-Output "{attempting_store_update}"
        $result = (Get-CimInstance{verbose_flag} -Namespace "Root\cimv2\mdm\dmmap" -ClassName "MDM_EnterpriseModernAppManagement_AppManagement01" -ErrorAction Stop |
        Invoke-CimMethod{verbose_flag} -MethodName UpdateScanMethod -ErrorAction Stop).ReturnValue

        if ($result -eq 0) {
            Write-Output "{update_completed}"
        } else {
            Write-Output "FAIL_PRIMARY_NONZERO:$result"
        }
    } catch {
        Write-Output "FAIL_PRIMARY_EXCEPTION:$($_.Exception.Message)"
    }"#;
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

    // Simplified translation approach with direct string literals for t!()
    fn get_translation(&self, key: &str) -> String {
        let translation = match key {
            "powershell_not_installed" => t!("Powershell is not installed"),
            "admin_privileges_required" => t!("Administrator privileges required - you will see a UAC prompt"),
            "starting_windows_update" => t!("Starting Windows Update..."),
            "windows_update_completed" => t!("Windows Update check completed"),
            "microsoft_store_update_failed" => t!("Microsoft Store update failed"),
            "attempting_open_store" => t!("Attempting to open Microsoft Store updates page..."),
            "failed_open_store" => t!("Failed to open Microsoft Store"),
            "opened_store_page" => t!("Opened Microsoft Store updates page. Please check for updates manually."),
            "attempting_reset_store" => t!("Attempting to reset Microsoft Store..."),
            "failed_reset_store" => t!("Failed to reset Microsoft Store"),
            "initiated_store_reset" => t!("Initiated Microsoft Store reset. Updates should begin shortly."),
            "scanning_for_updates" => t!("Scanning for updates..."),
            "processing_powershell_modules" => t!("Processing PowerShell modules..."),
            "gallery_not_accessible" => t!("PowerShell Gallery is not accessible. Module updates will be skipped."),
            "will_load_modules" => t!("Will still attempt to load existing modules"),
            "powershell_module_processing_complete" => t!("PowerShell module processing complete."),
            "powershell_modules_update_check_completed" => t!("PowerShell Modules update check completed"),
            "microsoft_store_update_check_completed" => t!("Microsoft Store update check completed"),
            "checking_connectivity" => t!("Checking connectivity to PowerShell Gallery..."),
            "gallery_accessible" => t!("PowerShell Gallery is accessible"),
            "gallery_not_accessible_full" => {
                t!("PowerShell Gallery is not accessible. Module updates will be skipped.")
            }
            "processing_module" => t!("Processing module: {moduleName}"),
            "unloading_module" => t!("Unloading module: {moduleName}"),
            "updating_module" => t!("Updating module: {moduleName}"),
            "retry_attempt" => t!("Retry attempt {attempt} of {max}..."),
            "update_failed" => t!("Failed to update module after multiple attempts"),
            "reloading_module" => t!("Reloading module: {moduleName}"),
            "import_success" => t!("Successfully imported module: {moduleName}"),
            "import_failed" => t!("Could not reload module: {moduleName} - {error}"),
            "process_failed" => t!("Failed to process module: {moduleName} - {error}"),
            "attempting_store_update" => t!("Attempting to update Microsoft Store apps using MDM method..."),
            // If it's a dynamic operation name completion message
            _ if key.ends_with("_check_completed") => {
                let op_name = &key[0..key.len() - 16]; // Remove "_check_completed"
                return format!("{} {}", op_name, t!("check_completed"));
            }
            _ => return key.to_string(),
        };

        self.clean_translation(&translation)
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
            .add_translation("checking_connectivity", self.get_translation("checking_connectivity"))
            .add_translation("gallery_accessible", self.get_translation("gallery_accessible"))
            .add_translation(
                "gallery_not_accessible",
                self.get_translation("gallery_not_accessible_full"),
            )
            .build(|t| self.clean_translation(t));

        // Create modules update script with cleaner builder pattern
        let update_modules_script = ScriptBuilder::new(scripts::MODULES_UPDATE_TEMPLATE)
            .add_translation("processing_module", "Processing module: {0}")
            .add_translation("unloading_module", "Unloading module: {0}")
            .add_translation("updating_module", "Updating module: {0}")
            .with_param("update_command", &update_command)
            .add_translation("retry_attempt", "Retry attempt {0} of {1}...")
            .add_translation("update_failed", "Failed to update module after multiple attempts")
            .add_translation("reloading_module", "Reloading module: {0}")
            .add_translation("import_success", "Successfully imported module: {0}")
            .add_translation("import_failed", "Could not reload module: {0}")
            .add_translation("process_failed", "Failed to process module: {0}")
            .build(|t| self.clean_translation(t));

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
            self.get_translation("processing_powershell_modules"),
            self.get_translation("gallery_not_accessible"),
            self.get_translation("will_load_modules"),
            self.get_translation("powershell_module_processing_complete"),
            self.get_translation("powershell_modules_update_check_completed")
        )
    }

    // Consolidated command creation with improved error handling
    fn create_command(&self, ctx: &ExecutionContext) -> Result<Executor> {
        let powershell = require_option(self.path.as_ref(), self.get_translation("powershell_not_installed"))?;

        Ok(if let Some(sudo) = ctx.sudo() {
            let mut cmd = ctx.run_type().execute(sudo);
            cmd.arg(powershell);
            cmd
        } else {
            ctx.run_type().execute(powershell)
        })
    }

    /// Helper method that returns PowerShell code to pause and wait for a keypress
    fn get_pause_code(&self) -> &str {
        r#"
# Keep window open if running in a separate window
if ($Host.Name -eq 'ConsoleHost' -and $Host.UI.RawUI.KeyAvailable -eq $false) {
    Write-Host "Press any key to continue..." -ForegroundColor Yellow
    $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
}"#
    }

    // Execute script with improved error handling
    fn execute_script(&self, ctx: &ExecutionContext, script: &str) -> Result<()> {
        // Check elevation status first
        self.check_elevation(ctx);

        // Check execution policy on Windows
        #[cfg(windows)]
        self.execution_policy_args_if_needed()?;

        // Append pause code to keep the PowerShell window open
        let script_with_pause = format!("{}\n{}", script, self.get_pause_code());

        // Create and execute command in one go
        self.create_command(ctx)?
            .args(Self::default_args())
            .arg(PS_COMMAND)
            .arg(script_with_pause)
            .status_checked()
    }

    // Extract elevation check to separate method to improve readability
    fn check_elevation(&self, ctx: &ExecutionContext) {
        if ctx.sudo().is_some() && !Self::is_process_elevated() && !self.uac_prompt_shown.get() {
            println!("{}", self.get_translation("admin_privileges_required"));
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
            println!("{}", self.get_translation("scanning_for_updates"));
        }

        // Execute the operation
        let result = operation();

        // Show completion message if operation succeeded and we're not elevating
        if result.is_ok() && !will_elevate {
            // Use operation-specific completed message based on the operation_name
            let completed_message = match operation_name {
                "PowerShell Modules" => self.get_translation("powershell_modules_update_check_completed"),
                "Windows Update" => self.get_translation("windows_update_completed"),
                "Microsoft Store" => self.get_translation("microsoft_store_update_check_completed"),
                _ => self.get_translation(&format!("{}_check_completed", operation_name.to_lowercase())),
            };
            println!("{}", completed_message);
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

    #[cfg(windows)]
    fn create_microsoft_store_script(powershell: &Powershell, verbose_flag: &str) -> String {
        ScriptBuilder::new(scripts::MS_STORE_UPDATE_TEMPLATE)
            .add_translation(
                "attempting_store_update",
                powershell.get_translation("attempting_store_update"),
            )
            .with_param("verbose_flag", verbose_flag)
            .add_translation(
                "update_completed",
                powershell.get_translation("microsoft_store_update_check_completed"),
            )
            .build(|t| powershell.clean_translation(t))
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
            self.get_translation("starting_windows_update"),
            verbose_flag,
            accept_flag,
            self.get_translation("windows_update_completed")
        );

        self.execute_operation(ctx, "Windows Update", || self.execute_script(ctx, &script))
    }

    // Improved Windows Store script creation with better error handling
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
        let ps_script = Powershell::create_microsoft_store_script(powershell, verbose_flag);

        powershell.execute_operation(ctx, "Microsoft Store", || {
            match powershell.execute_script(ctx, &ps_script) {
                Ok(_) => Ok(()),
                Err(e) => {
                    // Log error and try fallbacks
                    println!("{}: {}", powershell.get_translation("microsoft_store_update_failed"), e);

                    try_microsoft_store_fallbacks(powershell, ctx)?;

                    // Still return an error for proper status tracking
                    Err(color_eyre::eyre::eyre!(
                        "Primary Microsoft Store update method failed. Fallbacks attempted."
                    ))
                }
            }
        })
    }

    fn try_microsoft_store_fallbacks(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        // First fallback: open Microsoft Store updates page
        println!("{}", powershell.get_translation("attempting_open_store"));

        // Try to open the Microsoft Store updates page
        let store_script = r#"$Launcher = [Windows.System.Launcher,Windows.System,ContentType=WindowsRuntime];
                $Launcher::LaunchUriAsync([uri]'ms-windows-store://downloadsandupdates').GetAwaiter().GetResult()"#;

        if let Err(e) = powershell.execute_script(ctx, store_script) {
            println!("{}: {}", powershell.get_translation("failed_open_store"), e);
        } else {
            println!("{}", powershell.get_translation("opened_store_page"));
        }

        // Second fallback: wsreset
        println!("{}", powershell.get_translation("attempting_reset_store"));

        if let Err(e) = ctx.run_type().execute("wsreset.exe").arg("-i").status_checked() {
            println!("{}: {}", powershell.get_translation("failed_reset_store"), e);
        } else {
            println!("{}", powershell.get_translation("initiated_store_reset"));
        }

        Ok(())
    }
}
