use std::path::Path;
use std::{ffi::OsStr, process::Command};

use color_eyre::eyre::Result;
use etcetera::base_strategy::BaseStrategy;
use tracing::{debug, info};

use crate::command::CommandExt;
use crate::error::SkipStep;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::steps::git::RepoStep;
use crate::terminal::{print_separator, print_warning};
use crate::utils::{require, which};
use rust_i18n::t;

pub fn run_chocolatey(ctx: &ExecutionContext) -> Result<()> {
    let choco = require("choco")?;
    let yes = ctx.config().yes(Step::Chocolatey);

    print_separator("Chocolatey");

    let mut command = match ctx.sudo() {
        Some(sudo) => sudo.execute(ctx, &choco)?,
        None => ctx.execute(choco),
    };

    command.args(["upgrade", "all"]);

    if yes {
        command.arg("--yes");
    }

    command.status_checked()
}

pub fn run_winget(ctx: &ExecutionContext) -> Result<()> {
    let winget = require("winget")?;

    print_separator("winget");

    ctx.execute(&winget).args(["source", "update"]).status_checked()?;

    let mut command = if ctx.config().winget_use_sudo() {
        match ctx.sudo() {
            Some(sudo) => sudo.execute(ctx, &winget)?,
            None => ctx.execute(winget),
        }
    } else {
        ctx.execute(winget)
    };

    let mut args = vec!["upgrade", "--all"];
    if ctx.config().winget_silent_install() {
        args.push("--silent");
    }

    command.args(args).status_checked()?;

    Ok(())
}

pub fn run_scoop(ctx: &ExecutionContext) -> Result<()> {
    let scoop = require("scoop")?;

    print_separator("Scoop");

    ctx.execute(&scoop).args(["update"]).status_checked()?;
    ctx.execute(&scoop).args(["update", "*"]).status_checked()?;

    if ctx.config().cleanup() {
        ctx.execute(&scoop).args(["cleanup", "*"]).status_checked()?;
        ctx.execute(&scoop).args(["cache", "rm", "-a"]).status_checked()?
    }
    Ok(())
}

/// Updates drivers using Snappy Driver Installer Origin (SDIO).
///
/// SDIO is a free open-source tool for downloading and installing drivers.
/// It will be executed in script mode to automatically download missing driver packs
/// and install missing drivers with restore point creation when possible.
pub fn run_sdio(ctx: &ExecutionContext) -> Result<()> {
    let sdio = if let Some(configured_path) = ctx.config().sdio_path() {
        // Use configured path first
        require(configured_path)?
    } else {
        // Try to detect SDIO automatically using various methods
        detect_sdio()?
    };

    print_separator(t!("Snappy Driver Installer Origin"));

    // Create dedicated temp directory for SDIO operations
    let sdio_work_dir = std::env::temp_dir().join("topgrade_sdio");
    std::fs::create_dir_all(&sdio_work_dir).ok();

    // Create a dynamic SDIO script based on run mode
    let script_content = if ctx.run_type().dry() {
        // Dry-run script: analyze devices without installing
        format!(
            r#"# Topgrade SDIO Analysis Script
# This script analyzes the system for driver updates without installing

# Configure directories
extractdir {}
logdir {}

# Enable logging for dry-run analysis
logging on
{}

# Initialize and scan system
init

# Generate device analysis report
writedevicelist device_analysis.txt

# Select missing and better drivers for analysis
select missing better

# End without installation
echo Analysis complete - no drivers installed in dry-run mode
end
"#,
            sdio_work_dir.display(),
            sdio_work_dir.join("logs").display(),
            if ctx.config().verbose() {
                "debug on\nverbose 255"
            } else {
                "verbose 128"
            }
        )
    } else {
        // Real installation script
        format!(
            r#"# Topgrade SDIO Installation Script
# This script automatically updates drivers with safety measures

# Configure directories
extractdir {}
logdir {}

# Enable logging
logging on
{}

# Create restore point for safety
restorepoint Topgrade SDIO Driver Update

# Check for updates first
checkupdates

# Initialize and scan system
init

# Select missing and better drivers
select missing better

# Download and install selected drivers
install

# Generate final device report
writedevicelist final_device_report.txt

# End script
echo Driver installation complete
end
"#,
            sdio_work_dir.display(),
            sdio_work_dir.join("logs").display(),
            if ctx.config().verbose() {
                "debug on\nverbose 255"
            } else {
                "verbose 128"
            }
        )
    };

    // Write the script to temp directory
    let script_path = sdio_work_dir.join("topgrade_sdio_script.txt");
    std::fs::write(&script_path, script_content)
        .map_err(|e| SkipStep(format!("Failed to create SDIO script: {}", e)))?;

    // Build script-based command arguments (non-deprecated)
    let mut args = vec![format!("-script:{}", script_path.display())];

    // Add additional non-deprecated options
    args.extend_from_slice(&[
        "-nologfile".to_string(),   // We handle logging through the script
        "-nostamp".to_string(),     // Clean log format
        "-preservecfg".to_string(), // Don't overwrite config
    ]);

    // Log the command being executed for transparency
    debug!("SDIO command: {:?} {:?}", sdio, args);
    info!("Running SDIO script: {}", script_path.display());
    info!("SDIO working directory: {}", sdio_work_dir.display());

    let mut command = ctx.execute(&sdio);
    command.args(&args);
    command.current_dir(&sdio_work_dir);

    let result = command.status_checked();

    // Print separator after execution for clean output formatting
    print_separator("");

    result
}

/// Detects SDIO installation using multiple strategies based on SDIO documentation
fn detect_sdio() -> Result<std::path::PathBuf> {
    let is_64bit = std::env::consts::ARCH == "x86_64";

    // Strategy 1: Try configured or PATH-based executables first
    // Priority order based on SDIO documentation:
    // 1. Architecture-specific versioned executables (SDIO_x64_R*.exe, SDIO_R*.exe)
    // 2. Generic architecture executables (SDIO_x64.exe, SDIO.exe)
    // 3. Batch files (SDIO_auto.bat) - less reliable for automation

    let executable_patterns = if is_64bit {
        vec![
            "SDIO_x64_R*.exe", // 64-bit versioned (highest priority)
            "SDIO_x64.exe",    // 64-bit generic
            "SDIO_R*.exe",     // 32-bit versioned (fallback)
            "SDIO.exe",        // Generic executable
            "SDIO_auto.bat",   // Batch file (lowest priority)
            "sdio",            // Generic name for scoop/chocolatey
        ]
    } else {
        vec![
            "SDIO_R*.exe",   // 32-bit versioned (highest priority)
            "SDIO.exe",      // Generic executable
            "SDIO_auto.bat", // Batch file
            "sdio",          // Generic name
        ]
    };

    // Strategy 2: Try each pattern in PATH
    for pattern in &executable_patterns {
        if let Some(exe) = which(pattern) {
            return Ok(exe);
        }
    }

    // Strategy 3: Check common installation locations
    if let Some(exe) = check_common_locations(is_64bit) {
        return Ok(exe);
    }

    // Strategy 4: Use glob patterns as final fallback
    if let Some(exe) = find_sdio_by_pattern(is_64bit) {
        return Ok(exe);
    }

    Err(SkipStep(t!("SDIO (Snappy Driver Installer Origin) not found").to_string()).into())
}

/// Checks common SDIO installation locations
fn check_common_locations(is_64bit: bool) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    let possible_locations = [
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
    ];

    for location in &possible_locations {
        let base_path = PathBuf::from(location);
        if !base_path.exists() {
            continue;
        }

        // Try SDIO_auto.bat first
        let auto_bat = base_path.join("SDIO_auto.bat");
        if auto_bat.exists() {
            return Some(auto_bat);
        }

        // Try versioned executables
        if let Some(exe) = find_versioned_executable(&base_path, is_64bit) {
            return Some(exe);
        }
    }

    None
}

/// Finds versioned SDIO executables in a directory
fn find_versioned_executable(dir: &std::path::Path, is_64bit: bool) -> Option<std::path::PathBuf> {
    use std::fs;

    let Ok(entries) = fs::read_dir(dir) else {
        return None;
    };

    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        // Look for SDIO executables
        if name.starts_with("SDIO") && name.ends_with(".exe") {
            let path = entry.path();

            if is_64bit && name.contains("x64") {
                // Prefer 64-bit on 64-bit systems
                candidates.insert(0, path);
            } else if name.starts_with("SDIO_R") || name == "SDIO.exe" {
                // Add other versions
                candidates.push(path);
            }
        }
    }

    candidates.into_iter().next()
}

/// Uses glob patterns to find SDIO executables
fn find_sdio_by_pattern(is_64bit: bool) -> Option<std::path::PathBuf> {
    // This is a fallback - in practice, the other methods should find SDIO
    // But we keep this as a safety net
    let patterns = if is_64bit {
        vec!["SDIO_x64*.exe", "SDIO*.exe"]
    } else {
        vec!["SDIO_R*.exe", "SDIO.exe"]
    };

    for pattern in patterns {
        if let Some(exe) = which(pattern) {
            return Some(exe);
        }
    }

    None
}

pub fn update_wsl(ctx: &ExecutionContext) -> Result<()> {
    if !is_wsl_installed()? {
        return Err(SkipStep(t!("WSL not installed").to_string()).into());
    }

    let wsl = require("wsl")?;

    print_separator(t!("Update WSL"));

    let mut wsl_command = ctx.execute(wsl);
    wsl_command.args(["--update"]);

    if ctx.config().wsl_update_pre_release() {
        wsl_command.args(["--pre-release"]);
    }

    if ctx.config().wsl_update_use_web_download() {
        wsl_command.args(["--web-download"]);
    }
    wsl_command.status_checked()?;
    Ok(())
}

/// Detect if WSL is installed or not.
///
/// For WSL, we cannot simply check if command `wsl` is installed as on newer
/// versions of Windows (since windows 10 version 2004), this command is
/// installed by default.
///
/// If the command is installed and the user hasn't installed any Linux distros
/// on it, command `wsl -l` would print a help message and exit with failure, we
/// use this to check whether WSL is install or not.
fn is_wsl_installed() -> Result<bool> {
    if let Some(wsl) = which("wsl") {
        // Don't use `output_checked` as an execution failure log is not wanted
        #[allow(clippy::disallowed_methods)]
        let output = Command::new(wsl).arg("-l").output()?;
        let status = output.status;

        if status.success() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn get_wsl_distributions(wsl: &Path) -> Result<Vec<String>> {
    let output = Command::new(wsl).args(["--list", "-q"]).output_checked_utf8()?.stdout;
    Ok(output
        .lines()
        .filter(|s| !s.is_empty())
        .map(|x| x.replace(['\u{0}', '\r'], ""))
        .collect())
}

fn upgrade_wsl_distribution(wsl: &Path, dist: &str, ctx: &ExecutionContext) -> Result<()> {
    let topgrade = Command::new(wsl)
        .args(["-d", dist, "bash", "-lc", "which topgrade"])
        .output_checked_utf8()
        .map_err(|_| SkipStep(t!("Could not find Topgrade installed in WSL").to_string()))?
        .stdout // The normal output from `which topgrade` appends a newline, so we trim it here.
        .trim_end()
        .to_owned();

    let mut command = ctx.execute(wsl);

    // The `arg` method automatically quotes its arguments.
    // This means we can't append additional arguments to `topgrade` in WSL
    // by calling `arg` successively.
    //
    // For example:
    //
    // ```rust
    // command
    //  .args(["-d", dist, "bash", "-c"])
    //  .arg(format!("TOPGRADE_PREFIX={dist} exec {topgrade}"));
    // ```
    //
    // creates a command string like:
    // > `C:\WINDOWS\system32\wsl.EXE -d Ubuntu bash -c 'TOPGRADE_PREFIX=Ubuntu exec /bin/topgrade'`
    //
    // Adding the following:
    //
    // ```rust
    // command.arg("-v");
    // ```
    //
    // appends the next argument like so:
    // > `C:\WINDOWS\system32\wsl.EXE -d Ubuntu bash -c 'TOPGRADE_PREFIX=Ubuntu exec /bin/topgrade' -v`
    // which means `-v` isn't passed to `topgrade`.
    let mut args = String::new();
    if ctx.config().verbose() {
        args.push_str("-v");
    }

    command
        .args(["-d", dist, "bash", "-c"])
        .arg(format!("TOPGRADE_PREFIX={dist} exec {topgrade} {args}"));

    if ctx.config().yes(Step::Wsl) {
        command.arg("-y");
    }

    command.status_checked()
}

pub fn run_wsl_topgrade(ctx: &ExecutionContext) -> Result<()> {
    if !is_wsl_installed()? {
        return Err(SkipStep(t!("WSL not installed").to_string()).into());
    }

    let wsl = require("wsl")?;
    let wsl_distributions = get_wsl_distributions(&wsl)?;
    let mut ran = false;

    debug!("WSL distributions: {:?}", wsl_distributions);

    for distribution in wsl_distributions {
        let result = upgrade_wsl_distribution(&wsl, &distribution, ctx);
        debug!("Upgrading {:?}: {:?}", distribution, result);
        if let Err(e) = result {
            if e.is::<SkipStep>() {
                continue;
            }
        }
        ran = true
    }

    if ran {
        Ok(())
    } else {
        Err(SkipStep(t!("Could not find Topgrade in any WSL distribution").to_string()).into())
    }
}

pub fn windows_update(ctx: &ExecutionContext) -> Result<()> {
    let powershell = ctx.require_powershell()?;

    print_separator(t!("Windows Update"));

    if powershell.supports_windows_update() {
        powershell.windows_update(ctx)
    } else {
        print_warning(t!(
            "The PSWindowsUpdate PowerShell module isn't installed so Topgrade can't run Windows Update.\nInstall PSWindowsUpdate by running `Install-Module PSWindowsUpdate` in PowerShell."
        ));

        Err(SkipStep(t!("PSWindowsUpdate is not installed").to_string()).into())
    }
}

pub fn microsoft_store(ctx: &ExecutionContext) -> Result<()> {
    let powershell = ctx.require_powershell()?;

    print_separator(t!("Microsoft Store"));

    powershell.microsoft_store(ctx)
}

pub fn reboot(ctx: &ExecutionContext) -> Result<()> {
    // If this works, it won't return, but if it doesn't work, it may return a useful error
    // message.
    ctx.execute("shutdown.exe").args(["/R", "/T", "0"]).status_checked()
}

pub fn insert_startup_scripts(git_repos: &mut RepoStep) -> Result<()> {
    let startup_dir = crate::WINDOWS_DIRS
        .data_dir()
        .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
    for entry in std::fs::read_dir(&startup_dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) == Some("lnk") {
            if let Ok(lnk) = parselnk::Lnk::try_from(Path::new(&path)) {
                debug!("Startup link: {:?}", lnk);
                if let Some(path) = lnk.relative_path() {
                    git_repos.insert_if_repo(startup_dir.join(path));
                }
            }
        }
    }

    Ok(())
}
