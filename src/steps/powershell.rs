<<<<<<< HEAD
use std::cell::Cell;
#[cfg(windows)]
=======
>>>>>>> main
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
    pub fn new() -> Self {
        let path = which("pwsh").or_else(|| which("powershell")).filter(|_| !is_dumb());
<<<<<<< HEAD
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
=======
        let profile = path.as_ref().and_then(Self::get_profile);
        Powershell { path, profile }
>>>>>>> main
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

<<<<<<< HEAD
    /// Helper function to clean translated strings by removing locale prefixes
    fn clean_translation(&self, text: impl Into<String>) -> String {
        let text = text.into();
        // Remove locale prefixes like "en-GB." from translated strings
        text.find('.')
            .filter(|&idx| text.chars().take(idx).all(|c| c.is_ascii_alphabetic() || c == '-'))
            .map_or(text.clone(), |idx| text[idx + 1..].to_string())
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

        // Create sections of the script for better readability
        let check_gallery_script = self.create_gallery_check_script();
        let update_modules_script = self.create_modules_update_script(&update_command);

        format!(
            r#"Write-Host "{}" -ForegroundColor Cyan

{check_gallery_script}

if ($galleryAvailable) {{
    {update_modules_script}
}} else {{
    Write-Host "{}" -ForegroundColor Red
    # Continue with module loading anyway, as they might still work
    Write-Host "{}" -ForegroundColor Yellow
}}

Write-Host "{}" -ForegroundColor Green
Write-Host "{}" -ForegroundColor Green"#,
            self.clean_translation(t!("Processing PowerShell modules...")),
            self.clean_translation(t!("Unable to connect to PowerShell Gallery. Module updates skipped.")),
            self.clean_translation(t!("Will still attempt to load existing modules")),
            self.clean_translation(t!("PowerShell module processing complete.")),
            self.clean_translation(t!("PowerShell Modules update check completed"))
        )
    }

    /// Creates the gallery connectivity check part of the script
    fn create_gallery_check_script(&self) -> String {
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

    /// Creates the module update portion of the script
    fn create_modules_update_script(&self, update_command: &str) -> String {
        format!(
            r#"Get-Module -ListAvailable | Select-Object -Property Name -Unique | ForEach-Object {{
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
  }}"#,
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
            ))
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
            // First substitute the operation_name into the translation string, then clean it
            let completed_message = t!("{operation_name} check completed", operation_name = operation_name);
            println!("{}", self.clean_translation(completed_message));
        }

        result
=======
    fn get_profile(path: &PathBuf) -> Option<PathBuf> {
        Self::execute_with_command(path, &["-NoProfile", "-Command", "Split-Path $PROFILE"], |stdout| {
            Ok(stdout)
        })
        .ok() // Convert the Result<String> to Option<String>
        .and_then(|s| super::super::utils::PathExt::require(PathBuf::from(s)).ok())
    }

    fn execute_with_command<F>(path: &PathBuf, args: &[&str], f: F) -> Result<String>
    where
        F: FnOnce(String) -> Result<String>,
    {
        let output = Command::new(path).args(args).output_checked_utf8()?;
        let stdout = output.stdout.trim().to_string();
        f(stdout)
    }

    /// Builds a command with common arguments and optional sudo support.
    fn build_command_internal<'a>(
        &self,
        ctx: &'a ExecutionContext,
        additional_args: &[&str],
    ) -> Result<impl CommandExt + 'a> {
        let powershell = require_option(self.path.as_ref(), t!("Powershell is not installed").to_string())?;
        let executor = &mut ctx.run_type();
        let mut command = if let Some(sudo) = ctx.sudo() {
            let mut cmd = executor.execute(sudo);
            cmd.arg(powershell);
            cmd
        } else {
            executor.execute(powershell)
        };

        #[cfg(windows)]
        {
            // Check execution policy and return early if it's not set correctly
            self.execution_policy_args_if_needed()?;
        }

        command.args(Self::common_args()).args(additional_args);
        Ok(command)
>>>>>>> main
    }

    pub fn update_modules(&self, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("Powershell Modules Update"));
<<<<<<< HEAD

        self.execute_operation(ctx, "PowerShell Modules", || {
            let script = self.create_update_script(ctx);
            self.execute_script(ctx, &script)
        })
=======
        let mut cmd_args = vec!["Update-Module"];

        if ctx.config().verbose() {
            cmd_args.push("-Verbose");
        }
        if ctx.config().yes(Step::Powershell) {
            cmd_args.push("-Force");
        }
        println!("{}", t!("Updating modules..."));
        self.build_command_internal(ctx, &cmd_args)?.status_checked()
    }

    fn common_args() -> &'static [&'static str] {
        &["-NoProfile"]
    }

    #[cfg(windows)]
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

    #[cfg(windows)]
    fn is_execution_policy_set(&self, policy: &str) -> bool {
        if let Some(powershell) = &self.path {
            // These policies are ordered from most restrictive to least restrictive
            let valid_policies = ["Restricted", "AllSigned", "RemoteSigned", "Unrestricted", "Bypass"];

            // Find the index of our target policy
            let target_idx = valid_policies.iter().position(|&p| p == policy);

            let output = Command::new(powershell)
                .args(["-NoProfile", "-Command", "Get-ExecutionPolicy"])
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
>>>>>>> main
    }
}

#[cfg(windows)]
impl Powershell {
    pub fn supports_windows_update(&self) -> bool {
<<<<<<< HEAD
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
        let verbose_flag = if ctx.config().verbose() { " -Verbose" } else { "" };
        let accept_flag = if ctx.config().accept_all_windows_updates() {
            " -AcceptAll"
        } else {
            ""
        };

        // Build command with appropriate flags
        let script = format!(
            "Write-Output '{}'; Install-WindowsUpdate{}{} -Confirm:$false; Write-Output '{}'",
            self.clean_translation(t!("Starting Windows Update...")),
            verbose_flag,
            accept_flag,
            self.clean_translation(t!("Windows Update check completed"))
        );

        self.execute_operation(ctx, "Windows Update", || self.execute_script(ctx, &script))
=======
        windows::supports_windows_update(self)
    }

    pub fn windows_update(&self, ctx: &ExecutionContext) -> Result<()> {
        windows::windows_update(self, ctx)
>>>>>>> main
    }

    pub fn microsoft_store(&self, ctx: &ExecutionContext) -> Result<()> {
        windows::microsoft_store(self, ctx)
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

<<<<<<< HEAD
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
=======
    pub fn supports_windows_update(powershell: &Powershell) -> bool {
        powershell
            .path
            .as_ref()
            .map(|p| has_module(p, "PSWindowsUpdate"))
            .unwrap_or(false)
    }

    #[cfg(windows)]
    pub fn windows_update(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        debug_assert!(supports_windows_update(powershell));

        // Build the full command string
        let mut command_str = "Install-WindowsUpdate -Verbose".to_string();
        if ctx.config().accept_all_windows_updates() {
            command_str.push_str(" -AcceptAll");
        }

        // Pass the command string using the -Command flag
        powershell
            .build_command_internal(ctx, &["-Command", &command_str])?
            .status_checked()
    }

    pub fn microsoft_store(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        println!("{}", t!("Scanning for updates..."));
        let update_command = "Start-Process powershell -Verb RunAs -ArgumentList '-Command', \
            '(Get-CimInstance -Namespace \"Root\\cimv2\\mdm\\dmmap\" \
            -ClassName \"MDM_EnterpriseModernAppManagement_AppManagement01\" | \
            Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue'";

        powershell
            .build_command_internal(ctx, &["-Command", update_command])?
            .status_checked()
    }

    fn has_module(powershell: &PathBuf, command: &str) -> bool {
        Command::new(powershell)
            .args([
                "-NoProfile",
                "-Command",
                &format!("Get-Module -ListAvailable {}", command),
>>>>>>> main
            ])
            .output_checked_utf8()
            .map(|result| !result.stdout.is_empty())
            .unwrap_or(false)
    }
<<<<<<< HEAD

    pub fn microsoft_store(powershell: &Powershell, ctx: &ExecutionContext) -> Result<()> {
        let verbose_flag = if ctx.config().verbose() { " -Verbose" } else { "" };
        let ps_script = create_microsoft_store_script(powershell, verbose_flag);

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

    fn create_microsoft_store_script(powershell: &Powershell, verbose_flag: &str) -> String {
        format!(
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
        )
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
=======
>>>>>>> main
}
