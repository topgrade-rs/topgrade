use std::convert::TryFrom;
use std::path::Path;
use std::{ffi::OsStr, process::Command};

use color_eyre::eyre::Result;
use etcetera::base_strategy::BaseStrategy;
use tracing::debug;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::{print_separator, print_warning};
use crate::utils::{require, which};
use crate::{error::SkipStep, steps::git::Repositories};
use crate::{powershell, Step};

pub fn run_chocolatey(ctx: &ExecutionContext) -> Result<()> {
    let choco = require("choco")?;
    let yes = ctx.config().yes(Step::Chocolatey);

    print_separator("Chocolatey");

    let mut command = match ctx.sudo() {
        Some(sudo) => {
            let mut command = ctx.run_type().execute(sudo);
            command.arg(choco);
            command
        }
        None => ctx.run_type().execute(choco),
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

    if !ctx.config().enable_winget() {
        print_warning("Winget is disabled by default. Enable it by setting enable_winget=true in the [windows] section in the configuration.");
        return Err(SkipStep(String::from("Winget is disabled by default")).into());
    }

    ctx.run_type()
        .execute(winget)
        .args(["upgrade", "--all"])
        .status_checked()
}

pub fn run_scoop(ctx: &ExecutionContext) -> Result<()> {
    let scoop = require("scoop")?;

    print_separator("Scoop");

    ctx.run_type().execute(&scoop).args(["update"]).status_checked()?;
    ctx.run_type().execute(&scoop).args(["update", "*"]).status_checked()?;

    if ctx.config().cleanup() {
        ctx.run_type().execute(&scoop).args(["cleanup", "*"]).status_checked()?;
    }

    Ok(())
}

pub fn update_wsl(ctx: &ExecutionContext) -> Result<()> {
    if !is_wsl_installed()? {
        return Err(SkipStep("WSL not installed".to_string()).into());
    }

    let wsl = require("wsl")?;

    print_separator("Update WSL");

    let mut wsl_command = ctx.run_type().execute(wsl);
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
/// versions of Windows (since windows 10 version 2004), this commmand is
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
        .map_err(|_| SkipStep(String::from("Could not find Topgrade installed in WSL")))?
        .stdout // The normal output from `which topgrade` appends a newline, so we trim it here.
        .trim_end()
        .to_owned();

    let mut command = ctx.run_type().execute(wsl);

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
        return Err(SkipStep("WSL not installed".to_string()).into());
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
        Err(SkipStep(String::from("Could not find Topgrade in any WSL disribution")).into())
    }
}

pub fn windows_update(ctx: &ExecutionContext) -> Result<()> {
    let powershell = powershell::Powershell::windows_powershell();

    if powershell.supports_windows_update() {
        print_separator("Windows Update");
        return powershell.windows_update(ctx);
    }

    let usoclient = require("UsoClient")?;

    print_separator("Windows Update");
    println!("Running Windows Update. Check the control panel for progress.");
    ctx.run_type()
        .execute(&usoclient)
        .arg("ScanInstallWait")
        .status_checked()?;
    ctx.run_type().execute(&usoclient).arg("StartInstall").status_checked()
}

pub fn reboot() -> Result<()> {
    // If this works, it won't return, but if it doesn't work, it may return a useful error
    // message.
    Command::new("shutdown").args(["/R", "/T", "0"]).status_checked()
}

pub fn insert_startup_scripts(git_repos: &mut Repositories) -> Result<()> {
    let startup_dir = crate::WINDOWS_DIRS
        .data_dir()
        .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
    for entry in std::fs::read_dir(&startup_dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) == Some("lnk") {
            if let Ok(lnk) = parselnk::Lnk::try_from(Path::new(&path)) {
                debug!("Startup link: {:?}", lnk);
                if let Some(path) = lnk.relative_path() {
                    git_repos.insert_if_repo(&startup_dir.join(path));
                }
            }
        }
    }

    Ok(())
}
