use std::path::Path;
use std::{ffi::OsStr, process::Command};

use color_eyre::eyre::Result;
use etcetera::base_strategy::BaseStrategy;
use tracing::debug;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::{print_separator, print_warning};
use crate::utils::{require, which};
use crate::{error::SkipStep, steps::git::RepoStep};
use crate::{powershell, Step};
use rust_i18n::t;

// Common command arguments for package managers
const UPGRADE_ALL: &[&str] = &["upgrade", "all"];
const UPDATE: &[&str] = &["update"];
const CLEANUP: &[&str] = &["cleanup", "*"];
const CACHE_RM: &[&str] = &["cache", "rm", "-a"];
const WSL_UPDATE: &[&str] = &["--update"];
const WSL_LIST: &[&str] = &["--list", "-q"];

/// Helper function to run a command with common options for different package managers
fn run_command(ctx: &ExecutionContext, tool: &str, args: &[&str], step: Step) -> Result<()> {
    let tool_path = require(tool)?;
    let yes = ctx.config().yes(step);

    print_separator(tool);

    let mut command = match ctx.sudo() {
        Some(sudo) => {
            let mut command = ctx.run_type().execute(sudo);
            command.arg(tool_path);
            command
        }
        None => ctx.run_type().execute(tool_path),
    };

    command.args(args);

    if yes {
        command.arg("--yes");
    }

    command.status_checked()
}

/// Detect if WSL is installed or not.
///
/// For WSL, we cannot simply check if the `wsl` command is installed as on newer
/// versions of Windows (since Windows 10 version 2004), this command is
/// installed by default.
///
/// If the command is installed but the user hasn't installed any Linux distros,
/// running `wsl -l` will print a help message and exit with failure. We use this
/// behavior to check whether WSL is properly set up or not.
fn is_wsl_installed() -> Result<bool> {
    if let Some(wsl) = which("wsl") {
        let result = Command::new(wsl).arg("-l").output_checked();
        if let Ok(output) = result {
            return Ok(output.status.success());
        }
    }
    Ok(false)
}

/// Get a list of all installed WSL distributions on the system
fn get_wsl_distributions(wsl: &Path) -> Result<Vec<String>> {
    let output = Command::new(wsl).args(WSL_LIST).output_checked_utf8()?.stdout;
    Ok(output
        .lines()
        .filter(|s| !s.is_empty())
        .map(|x| x.replace(['\u{0}', '\r'], ""))
        .collect())
}

/// Run Topgrade inside a specific WSL distribution
fn upgrade_wsl_distribution(wsl: &Path, dist: &str, ctx: &ExecutionContext) -> Result<()> {
    let topgrade = find_topgrade_in_wsl(wsl, dist)?;
    let mut command = ctx.run_type().execute(wsl);
    let args = if ctx.config().verbose() { "-v" } else { "" };

    command
        .args(["-d", dist, "bash", "-c"])
        .arg(format!("TOPGRADE_PREFIX={dist} exec {topgrade} {args}"));

    if ctx.config().yes(Step::Wsl) {
        command.arg("-y");
    }

    command.status_checked()
}

/// Locate the Topgrade executable within a WSL distribution
fn find_topgrade_in_wsl(wsl: &Path, dist: &str) -> Result<String> {
    Ok(Command::new(wsl)
        .args(["-d", dist, "bash", "-lc", "which topgrade"])
        .output_checked_utf8()
        .map_err(|_| SkipStep(t!("Could not find Topgrade installed in WSL").to_string()))?
        .stdout
        .trim_end()
        .to_owned())
}

/// Run the Chocolatey package manager update
/// Chocolatey is a package manager for Windows similar to apt/yum on Linux
pub fn run_chocolatey(ctx: &ExecutionContext) -> Result<()> {
    run_command(ctx, "choco", UPGRADE_ALL, Step::Chocolatey)
}

/// Run the Windows Package Manager (winget) update
/// Winget is Microsoft's official package manager for Windows 10 and above
pub fn run_winget(ctx: &ExecutionContext) -> Result<()> {
    run_command(ctx, "winget", &["upgrade", "--all"], Step::Winget)
}

/// Run the Scoop package manager update and cleanup if configured
/// Scoop is a command-line installer for Windows, similar to Homebrew on macOS
pub fn run_scoop(ctx: &ExecutionContext) -> Result<()> {
    let scoop = require("scoop")?;

    print_separator("Scoop");

    execute_scoop_commands(ctx, &scoop)?;

    if ctx.config().cleanup() {
        cleanup_scoop(ctx, &scoop)?;
    }

    Ok(())
}

/// Execute the core Scoop update commands
fn execute_scoop_commands(ctx: &ExecutionContext, scoop: &Path) -> Result<()> {
    ctx.run_type().execute(scoop).args(UPDATE).status_checked()?;
    ctx.run_type().execute(scoop).args(["update", "*"]).status_checked()
}

/// Clean up old/unused Scoop packages and caches
fn cleanup_scoop(ctx: &ExecutionContext, scoop: &Path) -> Result<()> {
    ctx.run_type().execute(scoop).args(CLEANUP).status_checked()?;
    ctx.run_type().execute(scoop).args(CACHE_RM).status_checked()
}

/// Update the Windows Subsystem for Linux (WSL) core components
/// This updates the Linux kernel used by WSL and other system components
pub fn update_wsl(ctx: &ExecutionContext) -> Result<()> {
    if !is_wsl_installed()? {
        return Err(SkipStep(t!("WSL not installed").to_string()).into());
    }

    let wsl = require("wsl")?;

    print_separator(t!("Update WSL"));

    let mut wsl_command = ctx.run_type().execute(wsl);
    wsl_command.args(WSL_UPDATE);

    // Allow using pre-release WSL versions if configured
    if ctx.config().wsl_update_pre_release() {
        wsl_command.arg("--pre-release");
    }

    // Use web download instead of Microsoft Store for updates if configured
    if ctx.config().wsl_update_use_web_download() {
        wsl_command.arg("--web-download");
    }
    wsl_command.status_checked()?;
    Ok(())
}

/// Run Topgrade inside each installed WSL distribution
/// This allows updating Linux packages from within Windows
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

/// Run Windows Update using PowerShell
/// Uses the PSWindowsUpdate module if available, which provides better control
/// than the built-in Windows Update tools
pub fn windows_update(ctx: &ExecutionContext) -> Result<()> {
    let powershell = powershell::Powershell::windows_powershell();

    print_separator(t!("Windows Update"));

    if powershell.supports_windows_update() {
        println!("The installer will request to run as administrator, expect a prompt.");
        powershell.windows_update(ctx)
    } else {
        print_warning(t!(
            "Consider installing PSWindowsUpdate as the use of Windows Update via USOClient is not supported."
        ));
        Err(SkipStep(t!("USOClient not supported.").to_string()).into())
    }
}

/// Run updates for Microsoft Store apps
/// Updates installed apps from the Microsoft Store
pub fn microsoft_store(ctx: &ExecutionContext) -> Result<()> {
    let powershell = powershell::Powershell::windows_powershell();

    print_separator(t!("Microsoft Store"));

    powershell.microsoft_store(ctx)
}

/// Reboot the Windows system
/// Uses the shutdown command with restart flag
pub fn reboot() -> Result<()> {
    Command::new("shutdown").args(["/R", "/T", "0"]).status_checked()
}

/// Check startup scripts for Git repositories
/// This looks for shortcuts (.lnk files) in the Windows startup folder
/// and checks if they point to Git repositories
pub fn insert_startup_scripts(git_repos: &mut RepoStep) -> Result<()> {
    // Get the Windows startup folder path
    let startup_dir = crate::WINDOWS_DIRS
        .data_dir()
        .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");

    // Look for .lnk files (Windows shortcuts) in the startup folder
    for entry in std::fs::read_dir(&startup_dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) == Some("lnk") {
            // Parse the shortcut to get the target path
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
