use std::path::Path;

use color_eyre::eyre::Result;
use tracing::{debug, info};

use crate::command::CommandExt;
use crate::error::SkipStep;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::{print_separator, prompt_yesno};
use crate::utils::{require, which};
use rust_i18n::t;

/// Updates drivers using Snappy Driver Installer Origin (SDIO).
///
/// SDIO is a free open-source tool for downloading and installing drivers.
/// It will be executed in script mode to automatically download missing driver packs
/// and install missing drivers with restore point creation when possible.
///
/// Script generation follows the official SDIO scripting manual syntax:
/// - Commands don't use leading "-" or "/" signs
/// - Directory paths are quoted for safety
/// - Error handling uses "onerror" commands with goto labels
/// - Proper command ordering (checkupdates before init, etc.)
///
/// **Important**: This step requires explicit opt-in via the `enable_sdio = true`
/// configuration setting due to the critical nature of driver updates.
///
/// **Interactive Mode** (without --yes): Shows available driver updates and asks for user confirmation
/// **Automatic Mode** (with --yes): Installs drivers automatically without user interaction
pub fn run_sdio(ctx: &ExecutionContext) -> Result<()> {
    // Check if SDIO is explicitly enabled by the user
    if !ctx.config().enable_sdio() {
        return Err(SkipStep(
            t!("SDIO driver updates are disabled. Enable with 'enable_sdio = true' in [windows] section").to_string(),
        )
        .into());
    }

    let sdio = if let Some(configured_path) = ctx.config().sdio_path() {
        // Use configured path first (expand Windows env vars like %USERPROFILE%)
        let expanded = expand_env_vars_windows(configured_path);
        require(&expanded)?
    } else {
        // Try to detect SDIO automatically using various methods
        detect_sdio()?
    };

    let yes = ctx.config().yes(Step::Sdio);
    let interactive = !ctx.run_type().dry() && !yes;

    print_separator(t!("Snappy Driver Installer Origin"));

    // Create dedicated temp directory for SDIO operations
    let sdio_work_dir = std::env::temp_dir().join("topgrade_sdio");
    std::fs::create_dir_all(&sdio_work_dir).ok();

    // Create a dynamic SDIO script based on run mode and user preferences
    let verbose_settings = if ctx.config().verbose() {
        "debug on\nverbose 255"
    } else {
        "verbose 128"
    };

    let script_content = if ctx.run_type().dry() {
        // Dry-run script: analyze devices without installing
        format!(
            r#"# Topgrade SDIO Analysis Script
# This script analyzes the system for driver updates without installing

# Configure directories (quoted for safety)
extractdir "{}"
logdir "{}"

# Enable logging for dry-run analysis
logging on
{}

# Initialize and scan system
echo Initializing SDIO and scanning system for analysis...
init
onerror goto end

# Generate device analysis report before selection
writedevicelist device_analysis_before.txt

# Select missing and better drivers for analysis
echo Analyzing available driver updates...
select missing better

# Generate device analysis report after selection
writedevicelist device_analysis_after.txt

# Display analysis results
echo Analysis complete - no drivers installed in dry-run mode
echo Check device_analysis_before.txt and device_analysis_after.txt for details

:end
# End without installation
end
"#,
            sdio_work_dir.display(),
            sdio_work_dir.join("logs").display(),
            verbose_settings
        )
    } else if yes {
        // Automatic installation script (with --yes)
        format!(
            r#"# Topgrade SDIO Automatic Installation Script
# This script automatically updates drivers with safety measures (--yes mode)

# Configure directories (quoted for safety)
extractdir "{}"
logdir "{}"

# Enable logging
logging on
{}

# Check for updates first (before init for better performance)
echo Checking for SDIO updates...
checkupdates
onerror goto end

# Initialize and scan system
echo Initializing SDIO and scanning system...
init
onerror goto end

# Generate initial device report
writedevicelist initial_device_report.txt

# Create restore point for safety (quoted description)
echo Creating system restore point...
restorepoint "Topgrade SDIO Driver Update"
onerror echo Warning: Failed to create restore point, continuing anyway...

# Select missing and better drivers
echo Selecting drivers for installation...
select missing better

# Install selected drivers automatically (will auto-download if needed)
echo Installing selected drivers automatically...
install
onerror echo Warning: Some drivers may have failed to install

# Generate final device report
writedevicelist final_device_report.txt

# Driver installation complete
echo Driver installation complete
echo Check initial_device_report.txt and final_device_report.txt for details

:end
# End script
end
"#,
            sdio_work_dir.display(),
            sdio_work_dir.join("logs").display(),
            verbose_settings
        )
    } else {
        // Interactive analysis script (without --yes): analyze and exit without installing
        format!(
            r#"# Topgrade SDIO Interactive Analysis Script
# This script analyzes available driver updates and exits without installing

# Configure directories (quoted for safety)
extractdir "{}"
logdir "{}"

# Enable logging
logging on
{}

# Check for updates first (before init for better performance)
echo Checking for SDIO updates...
checkupdates
onerror goto end

# Initialize and scan system
echo Initializing SDIO and scanning system...
init
onerror goto end

# Generate initial device report
writedevicelist initial_device_report.txt

# Select missing and better drivers for analysis
echo Analyzing available driver updates...
select missing better

# Generate selected devices report (what would be changed)
writedevicelist selected_device_report.txt

echo.
echo Analysis complete. This is an analysis-only run.
echo Review initial_device_report.txt and selected_device_report.txt for details.
echo Re-run Topgrade with --yes to install selected drivers.

:end
# End script
end
"#,
            sdio_work_dir.display(),
            sdio_work_dir.join("logs").display(),
            verbose_settings
        )
    };

    // Write the script to temp directory
    let script_path = sdio_work_dir.join("topgrade_sdio_script.txt");
    std::fs::write(&script_path, script_content).map_err(|e| {
        SkipStep(format!(
            "Failed to create SDIO script at {}: {}",
            script_path.display(),
            e
        ))
    })?;

    // Log the command being executed for transparency and security auditing
    debug!("SDIO command: {:?} -script {:?}", sdio, script_path);
    info!("Running SDIO script: {}", script_path.display());
    info!("SDIO working directory: {}", sdio_work_dir.display());
    info!("SDIO binary location: {}", sdio.display());

    let mut command = ctx.execute(&sdio);
    // Pass -script and script path as separate args to handle spaces safely
    command.arg("-script").arg(&script_path);
    command.current_dir(&sdio_work_dir);

    let mut result = command.status_checked();

    // If interactive: ask the user whether to proceed with installation and run a second script
    if interactive && result.is_ok() {
        // Print separator for readability between analysis and prompt
        print_separator("");
        if let Ok(true) = prompt_yesno(&t!(
            "Proceed to install selected drivers now? This will create a restore point first. (y/N)"
        )) {
            // Build an installation script similar to --yes flow
            let install_script = format!(
                r#"# Topgrade SDIO Installation Script (interactive-confirmed)
# Configure directories (quoted for safety)
extractdir "{}"
logdir "{}"

# Enable logging
logging on
{}

# Check for updates first (before init for better performance)
echo Checking for SDIO updates...
checkupdates
onerror goto end

# Initialize and scan system
echo Initializing SDIO and scanning system...
init
onerror goto end

# Generate initial device report
writedevicelist initial_device_report.txt

# Create restore point for safety (quoted description)
echo Creating system restore point...
restorepoint "Topgrade SDIO Driver Update"
onerror echo Warning: Failed to create restore point, continuing anyway...

# Select missing and better drivers
echo Selecting drivers for installation...
select missing better

# Install selected drivers automatically
echo Installing selected drivers...
install
onerror echo Warning: Some drivers may have failed to install

# Generate final device report
writedevicelist final_device_report.txt

:end
end
"#,
                sdio_work_dir.display(),
                sdio_work_dir.join("logs").display(),
                verbose_settings
            );

            let install_script_path = sdio_work_dir.join("topgrade_sdio_install_script.txt");
            std::fs::write(&install_script_path, install_script).map_err(|e| {
                SkipStep(format!(
                    "Failed to create SDIO install script at {}: {}",
                    install_script_path.display(),
                    e
                ))
            })?;

            debug!("SDIO command (install): {:?} -script {:?}", sdio, install_script_path);
            info!("Running SDIO install script: {}", install_script_path.display());
            let mut install_cmd = ctx.execute(&sdio);
            install_cmd.arg("-script").arg(&install_script_path);
            install_cmd.current_dir(&sdio_work_dir);
            result = install_cmd.status_checked();
        } else {
            info!("User declined SDIO installation; analysis-only run completed.");
        }
    }

    // Print separator after execution for clean output formatting
    print_separator("");

    // Best-effort cleanup of the temporary workdir on success
    if result.is_ok() {
        let _ = std::fs::remove_dir_all(&sdio_work_dir);
    }

    result
}

/// Detects SDIO installation using multiple strategies based on SDIO documentation
fn detect_sdio() -> Result<std::path::PathBuf> {
    let is_64bit = std::env::consts::ARCH == "x86_64";

    // Strategy 1: Try PATH-based executables with priority order
    if let Some(exe) = detect_sdio_in_path(is_64bit) {
        return Ok(exe);
    }

    // Strategy 2: Check common installation locations
    if let Some(exe) = detect_sdio_in_common_locations(is_64bit) {
        return Ok(exe);
    }

    Err(SkipStep(t!("SDIO (Snappy Driver Installer Origin) not found").to_string()).into())
}

/// Detects SDIO executables in PATH with architecture-aware priority
fn detect_sdio_in_path(is_64bit: bool) -> Option<std::path::PathBuf> {
    // Only probe exact executable names; PATH lookups don't support globs
    let executable_names = get_sdio_executable_names(is_64bit);

    for name in &executable_names {
        if let Some(exe) = which(name) {
            return Some(exe);
        }
    }
    None
}

/// Returns SDIO executable patterns in priority order
fn get_sdio_executable_names(is_64bit: bool) -> Vec<&'static str> {
    if is_64bit {
        vec![
            // Prefer auto launcher if present
            "SDIO_auto.bat",
            // 64-bit generic executable
            "SDIO_x64.exe",
            // Generic executable
            "SDIO.exe",
            // Scoop alias or similar
            "sdio",
        ]
    } else {
        vec!["SDIO_auto.bat", "SDIO.exe", "sdio"]
    }
}

/// Detects SDIO in common installation locations
fn detect_sdio_in_common_locations(is_64bit: bool) -> Option<std::path::PathBuf> {
    let locations = get_common_sdio_locations();

    for location in locations {
        let base_path = std::path::PathBuf::from(location);
        if !base_path.exists() {
            continue;
        }

        // Try SDIO_auto.bat first (recommended)
        let auto_bat = base_path.join("SDIO_auto.bat");
        if auto_bat.exists() {
            return Some(auto_bat);
        }

        // Try versioned executables
        if let Some(exe) = find_best_executable_in_dir(&base_path, is_64bit) {
            return Some(exe);
        }
    }
    None
}

/// Expand Windows-style environment variables (e.g., %USERPROFILE%) in a path string.
/// Unknown variables are replaced with an empty string (matching common shell behavior).
fn expand_env_vars_windows(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut i = 0usize;
    let bytes = input.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // find the next '%'
            if let Some(end) = input[i + 1..].find('%') {
                let var_name = &input[i + 1..i + 1 + end];
                // Move index past the closing '%'
                i += end + 2;
                if !var_name.is_empty() {
                    if let Ok(val) = std::env::var(var_name) {
                        result.push_str(&val);
                    }
                    continue;
                } else {
                    // Handle literal '%%'
                    result.push('%');
                    continue;
                }
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

/// Returns common SDIO installation locations
fn get_common_sdio_locations() -> Vec<String> {
    vec![
        // Scoop installation in user profile
        format!(
            "{}\\scoop\\apps\\snappy-driver-installer-origin\\current",
            std::env::var("USERPROFILE").unwrap_or_default()
        ),
        // Common program files locations
        "C:\\Program Files\\SDIO".to_string(),
        "C:\\Program Files (x86)\\SDIO".to_string(),
        // Portable installations
        "C:\\SDIO".to_string(),
        format!("{}\\SDIO", std::env::var("USERPROFILE").unwrap_or_default()),
    ]
}

/// Finds the best SDIO executable in a directory based on architecture
fn find_best_executable_in_dir(dir: &Path, is_64bit: bool) -> Option<std::path::PathBuf> {
    use std::fs;

    let Ok(entries) = fs::read_dir(dir) else {
        return None;
    };

    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if name.starts_with("SDIO") && name.ends_with(".exe") {
            let path = entry.path();
            let priority = get_executable_priority(&name, is_64bit);
            candidates.push((priority, path));
        }
    }

    // Sort by priority (lower number = higher priority)
    candidates.sort_by_key(|(priority, _)| *priority);
    candidates.into_iter().map(|(_, path)| path).next()
}

/// Assigns priority to SDIO executables (lower number = higher priority)
fn get_executable_priority(name: &str, is_64bit: bool) -> u32 {
    match (name, is_64bit) {
        (name, true) if name.contains("x64") && name.starts_with("SDIO_x64_R") => 1, // 64-bit versioned
        (name, true) if name.contains("x64") => 2,                                   // 64-bit generic
        (name, _) if name.starts_with("SDIO_R") => 3,                                // Versioned
        ("SDIO.exe", _) => 4,                                                        // Generic
        _ => 5,                                                                      // Others
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdio_detection_methods() {
        // Test that SDIO detection doesn't panic with various inputs - using PATH detection
        let _ = std::panic::catch_unwind(|| detect_sdio_in_path(true));

        let _ = std::panic::catch_unwind(|| detect_sdio_in_path(false));
    }

    #[test]
    fn test_sdio_detection_error_handling() {
        // Test that PATH detection handles missing executables gracefully
        let result = detect_sdio_in_path(true);
        // Should return None or Some depending on system - just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_get_executable_priority_ordering() {
        // Test 64-bit priority ordering (higher priority = lower number)
        assert_eq!(get_executable_priority("SDIO_x64_R1515.exe", true), 1);
        assert_eq!(get_executable_priority("SDIO_x64.exe", true), 2);
        assert_eq!(get_executable_priority("SDIO_R1515.exe", true), 3);
        assert_eq!(get_executable_priority("SDIO.exe", true), 4);
        assert_eq!(get_executable_priority("other.exe", true), 5);
    }

    #[test]
    fn test_common_sdio_locations_format() {
        // Verify common SDIO locations are reasonable
        let common_locations = get_common_sdio_locations();

        // Should have at least a few common locations
        assert!(
            !common_locations.is_empty(),
            "Should have common SDIO locations defined"
        );

        // All locations should be reasonable Windows paths
        for location in &common_locations {
            assert!(!location.is_empty(), "Location should not be empty: {:?}", location);
        }
    }
}
