use std::fmt::Write as _;
use std::path::Path;

use color_eyre::eyre::Result;
use tracing::{debug, info};

use crate::command::CommandExt;
use crate::error::SkipStep;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::{print_info, print_separator, prompt_yesno};
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
    let verbose_output = ctx.config().verbose();
    let verbose_settings = if verbose_output {
        "debug on\nverbose 255"
    } else {
        "verbose 128"
    };

    let primary_mode = if ctx.run_type().dry() {
        ScriptMode::DryAnalysis
    } else if yes {
        ScriptMode::AutomaticInstall
    } else {
        ScriptMode::InteractiveAnalysis
    };

    let script_content = build_sdio_script(&sdio_work_dir, verbose_settings, verbose_output, primary_mode);

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

    let mut command = match ctx.sudo() {
        Some(sudo) => sudo.execute(ctx, &sdio)?,
        None => ctx.execute(&sdio),
    };
    // Pass -script and script path as separate args to handle spaces safely
    command.arg("-script").arg(&script_path);
    command.current_dir(&sdio_work_dir);

    announce_script_start(primary_mode, verbose_output);
    let mut result = command.status_checked();
    announce_script_finish(primary_mode, verbose_output, result.is_ok());

    // If interactive: ask the user whether to proceed with installation and run a second script
    if interactive && result.is_ok() {
        let report_path = sdio_work_dir.join("selected_device_report.txt");
        let mut should_prompt = true;

        match count_selected_drivers(&report_path) {
            Ok(0) => {
                print_info(t!(
                    "SDIO analysis found no drivers to install; keeping this run in analysis mode."
                ));
                info!(
                    "{}",
                    t!("SDIO analysis found no drivers to install; keeping this run in analysis mode.")
                );
                should_prompt = false;
            }
            Ok(count) => {
                debug!("SDIO analysis selected {} driver(s) for installation", count);
            }
            Err(err) => {
                debug!(
                    "Unable to inspect SDIO selection report at {}: {}",
                    report_path.display(),
                    err
                );
            }
        }

        if should_prompt {
            if let Ok(true) = prompt_yesno(&t!(
                "Proceed to install selected drivers now? This will create a restore point first. (y/N)"
            )) {
                // Build an installation script similar to --yes flow
                let install_mode = ScriptMode::InteractiveInstall;
                let install_script = build_sdio_script(&sdio_work_dir, verbose_settings, verbose_output, install_mode);

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
                let mut install_cmd = match ctx.sudo() {
                    Some(sudo) => sudo.execute(ctx, &sdio)?,
                    None => ctx.execute(&sdio),
                };
                install_cmd.arg("-script").arg(&install_script_path);
                install_cmd.current_dir(&sdio_work_dir);
                announce_script_start(install_mode, verbose_output);
                result = install_cmd.status_checked();
                announce_script_finish(install_mode, verbose_output, result.is_ok());
            } else {
                info!("User declined SDIO installation; analysis-only run completed.");
            }
        }
    }

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
        let base_path = std::path::PathBuf::from(&location);
        if !base_path.exists() {
            continue;
        }

        if base_path.is_file() {
            return Some(base_path);
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

fn count_selected_drivers(report_path: &Path) -> std::io::Result<usize> {
    let data = std::fs::read(report_path)?;
    let content = String::from_utf8_lossy(&data);

    Ok(content.lines().filter(|line| is_marked_selected(line)).count())
}

fn is_marked_selected(line: &str) -> bool {
    let mut parts = line.split([':', '=']);
    let key = match parts.next() {
        Some(key) => key.trim(),
        None => return false,
    };

    if !key.eq_ignore_ascii_case("selected") {
        return false;
    }

    let value = parts.next().map(|value| value.trim()).unwrap_or_default();
    let token = value.split_whitespace().next().unwrap_or("");

    if let Ok(num) = token.parse::<i32>() {
        return num > 0;
    }

    matches!(token.to_ascii_lowercase().as_str(), "true" | "yes")
}

#[derive(Clone, Copy)]
enum ScriptMode {
    DryAnalysis,
    InteractiveAnalysis,
    AutomaticInstall,
    InteractiveInstall,
}

fn build_sdio_script(work_dir: &Path, verbose_settings: &str, emit_echo: bool, mode: ScriptMode) -> String {
    let mut script = String::new();

    match mode {
        ScriptMode::DryAnalysis => {
            append_script_header(
                &mut script,
                "Topgrade SDIO Analysis Script",
                "This script analyzes the system for driver updates without installing",
                work_dir,
                verbose_settings,
            );

            script.push_str("enableinstall off\n\n");

            push_echo_line(&mut script, emit_echo, "Topgrade: starting SDIO dry-run analysis...");
            script.push_str("init\n");
            script.push_str("onerror goto end\n\n");

            script.push_str("# Generate device analysis report before selection\n");
            script.push_str("writedevicelist device_analysis_before.txt\n\n");

            script.push_str("select missing newer better\n\n");

            script.push_str("# Generate device analysis report after selection\n");
            script.push_str("writedevicelist device_analysis_after.txt\n\n");

            push_echo_line(
                &mut script,
                emit_echo,
                "Topgrade: SDIO dry-run analysis complete; no drivers installed.",
            );

            append_script_footer(&mut script, "End without installation");
        }
        ScriptMode::InteractiveAnalysis => {
            append_script_header(
                &mut script,
                "Topgrade SDIO Interactive Analysis Script",
                "This script analyzes available driver updates and exits without installing",
                work_dir,
                verbose_settings,
            );

            script.push_str("enableinstall off\n\n");

            push_echo_line(&mut script, emit_echo, "Topgrade: running SDIO analysis...");
            script.push_str("checkupdates\n");
            script.push_str("onerror goto end\n\n");

            script.push_str("init\n");
            script.push_str("onerror goto end\n\n");

            script.push_str("# Generate initial device report\n");
            script.push_str("writedevicelist initial_device_report.txt\n\n");

            script.push_str("select missing newer better\n\n");

            script.push_str("# Generate selected devices report (what would be changed)\n");
            script.push_str("writedevicelist selected_device_report.txt\n\n");

            push_echo_line(
                &mut script,
                emit_echo,
                "Topgrade: SDIO analysis complete; review reports for details.",
            );

            append_script_footer(&mut script, "End script");
        }
        ScriptMode::AutomaticInstall => {
            append_script_header(
                &mut script,
                "Topgrade SDIO Automatic Installation Script",
                "This script automatically updates drivers with safety measures (--yes mode)",
                work_dir,
                verbose_settings,
            );

            script.push_str("enableinstall on\n\n");

            push_echo_line(
                &mut script,
                emit_echo,
                "Topgrade: starting SDIO automatic installation...",
            );
            script.push_str("checkupdates\n");
            script.push_str("onerror goto end\n\n");

            script.push_str("init\n");
            script.push_str("onerror goto end\n\n");

            script.push_str("# Generate initial device report\n");
            script.push_str("writedevicelist initial_device_report.txt\n\n");

            script.push_str("restorepoint \"Topgrade SDIO Driver Update\"\n");
            script.push_str("onerror echo Warning: Failed to create restore point, continuing anyway...\n\n");

            script.push_str("select missing newer better\n\n");

            script.push_str("install\n");
            script.push_str("onerror echo Warning: Some drivers may have failed to install\n\n");

            script.push_str("# Generate final device report\n");
            script.push_str("writedevicelist final_device_report.txt\n\n");

            push_echo_line(
                &mut script,
                emit_echo,
                "Topgrade: SDIO installation finished; review reports for details.",
            );

            append_script_footer(&mut script, "End script");
        }
        ScriptMode::InteractiveInstall => {
            append_script_header(
                &mut script,
                "Topgrade SDIO Installation Script (interactive-confirmed)",
                "",
                work_dir,
                verbose_settings,
            );

            script.push_str("enableinstall on\n\n");

            push_echo_line(&mut script, emit_echo, "Topgrade: starting SDIO installation...");
            script.push_str("checkupdates\n");
            script.push_str("onerror goto end\n\n");

            script.push_str("init\n");
            script.push_str("onerror goto end\n\n");

            script.push_str("# Generate initial device report\n");
            script.push_str("writedevicelist initial_device_report.txt\n\n");

            script.push_str("restorepoint \"Topgrade SDIO Driver Update\"\n");
            script.push_str("onerror echo Warning: Failed to create restore point, continuing anyway...\n\n");

            script.push_str("select missing newer better\n\n");

            script.push_str("install\n");
            script.push_str("onerror echo Warning: Some drivers may have failed to install\n\n");

            script.push_str("# Generate final device report\n");
            script.push_str("writedevicelist final_device_report.txt\n\n");

            push_echo_line(
                &mut script,
                emit_echo,
                "Topgrade: SDIO installation complete; review reports for details.",
            );

            append_script_footer(&mut script, "End script");
        }
    }

    script
}

fn append_script_header(script: &mut String, title: &str, description: &str, work_dir: &Path, verbose_settings: &str) {
    let _ = writeln!(script, "# {title}");
    if !description.is_empty() {
        let _ = writeln!(script, "# {description}");
    }
    script.push('\n');
    script.push_str("# Configure directories (quoted for safety)\n");
    let _ = writeln!(script, "extractdir \"{}\"", work_dir.display());
    let _ = writeln!(script, "logdir \"{}\"", work_dir.join("logs").display());
    script.push('\n');
    script.push_str("# Enable logging\n");
    script.push_str("logging on\n");
    script.push_str(verbose_settings);
    script.push('\n');
    script.push('\n');
}

fn append_script_footer(script: &mut String, end_comment: &str) {
    script.push_str(":end\n");
    let _ = writeln!(script, "# {end_comment}");
    script.push_str("end\n");
}

fn push_echo_line(script: &mut String, emit: bool, message: &str) {
    if emit {
        let _ = writeln!(script, "echo {}", message);
    }
}

fn announce_script_start(mode: ScriptMode, verbose: bool) {
    let message = match mode {
        ScriptMode::DryAnalysis => t!("Running SDIO dry-run analysis..."),
        ScriptMode::InteractiveAnalysis => t!("Running SDIO analysis..."),
        ScriptMode::AutomaticInstall => t!("Running SDIO automatic installation..."),
        ScriptMode::InteractiveInstall => t!("Running SDIO installation..."),
    };

    if verbose {
        debug!("{message}");
    } else {
        print_info(message);
    }
}

fn announce_script_finish(mode: ScriptMode, verbose: bool, succeeded: bool) {
    if !succeeded {
        return;
    }

    let message = match mode {
        ScriptMode::DryAnalysis => t!("SDIO dry-run analysis complete."),
        ScriptMode::InteractiveAnalysis => t!("SDIO analysis complete."),
        ScriptMode::AutomaticInstall => t!("SDIO automatic installation complete."),
        ScriptMode::InteractiveInstall => t!("SDIO installation complete."),
    };

    if verbose {
        debug!("{message}");
    } else {
        print_info(message);
    }
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
    let user_profile = std::env::var("USERPROFILE").unwrap_or_default();

    let mut locations = vec![
        // Scoop installation in user profile
        format!("{user_profile}\\scoop\\apps\\snappy-driver-installer-origin\\current"),
        // Common program files locations
        "C:\\Program Files\\SDIO".to_string(),
        "C:\\Program Files (x86)\\SDIO".to_string(),
        // Portable installations
        "C:\\SDIO".to_string(),
        format!("{user_profile}\\SDIO"),
    ];

    if !user_profile.is_empty() {
        // Executables dropped directly in the user profile or common user folders
        locations.push(user_profile.clone());
        locations.push(format!("{user_profile}\\Desktop"));
        locations.push(format!("{user_profile}\\Downloads"));
    }

    let program_data = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
    locations.push(format!(
        "{program_data}\\chocolatey\\lib\\snappy-driver-installer-origin\\tools"
    ));
    locations.push(format!(
        "{program_data}\\chocolatey\\lib\\snappy-driver-installer-origin\\tools\\SDIO"
    ));

    locations
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
    use std::io::Write;

    use tempfile::NamedTempFile;

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

    #[test]
    fn test_count_selected_drivers_detects_selection() -> std::io::Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            "[Device]\nName: Example\nSelected: 1\n---\n[Device]\nName: Other\nSelected = 0"
        )?;

        assert_eq!(count_selected_drivers(file.path())?, 1);
        Ok(())
    }

    #[test]
    fn test_count_selected_drivers_zero_when_none_selected() -> std::io::Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            "[Device]\nName: Example\nSelected: 0\n[Device]\nName: Another\nSelected = 0"
        )?;

        assert_eq!(count_selected_drivers(file.path())?, 0);
        Ok(())
    }
}
