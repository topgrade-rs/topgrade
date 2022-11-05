use std::convert::TryFrom;
use std::path::Path;
use std::{ffi::OsStr, process::Command};

use anyhow::Result;
use log::debug;

use crate::execution_context::ExecutionContext;
use crate::executor::{CommandExt, RunType};
use crate::terminal::{print_separator, print_warning};
use crate::utils::require;
use crate::{error::SkipStep, steps::git::Repositories};
use crate::{powershell, Step};

pub fn run_chocolatey(ctx: &ExecutionContext) -> Result<()> {
    let choco = require("choco")?;
    let yes = ctx.config().yes(Step::Chocolatey);

    print_separator("Chocolatey");

    let mut cmd = &choco;
    let mut args = vec!["upgrade", "all"];

    if let Some(sudo) = ctx.sudo() {
        cmd = sudo;
        args.insert(0, "choco");
    }

    let mut command = ctx.run_type().execute(&cmd);

    command.args(&args);

    if yes {
        command.arg("--yes");
    }

    command.check_run()
}

pub fn run_winget(ctx: &ExecutionContext) -> Result<()> {
    let winget = require("winget")?;

    print_separator("winget");

    if !ctx.config().enable_winget() {
        print_warning("Winget is disabled by default. Enable it by setting enable_winget=true in the [windows] section in the configuration.");
        return Err(SkipStep(String::from("Winget is disabled by default")).into());
    }

    ctx.run_type().execute(&winget).args(&["upgrade", "--all"]).check_run()
}

pub fn run_scoop(cleanup: bool, run_type: RunType) -> Result<()> {
    let scoop = require("scoop")?;

    print_separator("Scoop");

    run_type.execute(&scoop).args(&["update"]).check_run()?;
    run_type.execute(&scoop).args(&["update", "*"]).check_run()?;

    if cleanup {
        run_type.execute(&scoop).args(&["cleanup", "*"]).check_run()?;
    }

    Ok(())
}

fn get_wsl_distributions(wsl: &Path) -> Result<Vec<String>> {
    let output = Command::new(wsl).args(&["--list", "-q"]).check_output()?;
    Ok(output
        .lines()
        .filter(|s| !s.is_empty())
        .map(|x| x.replace('\u{0}', "").replace('\r', ""))
        .collect())
}

fn upgrade_wsl_distribution(wsl: &Path, dist: &str, ctx: &ExecutionContext) -> Result<()> {
    let topgrade = Command::new(&wsl)
        .args(&["-d", dist, "bash", "-lc", "which topgrade"])
        .check_output()
        .map_err(|_| SkipStep(String::from("Could not find Topgrade installed in WSL")))?;

    let mut command = ctx.run_type().execute(&wsl);
    command
        .args(&["-d", dist, "bash", "-c"])
        .arg(format!("TOPGRADE_PREFIX={} exec {}", dist, topgrade));

    if ctx.config().yes(Step::Wsl) {
        command.arg("-y");
    }

    command.check_run()
}

pub fn run_wsl_topgrade(ctx: &ExecutionContext) -> Result<()> {
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
    ctx.run_type().execute(&usoclient).arg("ScanInstallWait").check_run()?;
    ctx.run_type().execute(&usoclient).arg("StartInstall").check_run()
}

pub fn reboot() {
    Command::new("shutdown").args(&["/R", "/T", "0"]).spawn().ok();
}

pub fn insert_startup_scripts(ctx: &ExecutionContext, git_repos: &mut Repositories) -> Result<()> {
    let startup_dir = ctx
        .base_dirs()
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
