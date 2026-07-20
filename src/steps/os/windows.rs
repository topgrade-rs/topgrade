use std::path::Path;
use std::{ffi::OsStr, process::Command};

use color_eyre::eyre::Result;
use etcetera::base_strategy::BaseStrategy;
use rust_i18n::t;
use tracing::debug;

use crate::command::CommandExt;
use crate::config::UpdatesAutoReboot;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::{print_separator, print_warning};
use crate::utils::{require, which};
use crate::{error::SkipStep, steps::git::RepoStep};

pub fn run_chocolatey(ctx: &ExecutionContext) -> Result<()> {
    let choco = require("choco")?;
    let yes = ctx.config().yes(Step::Chocolatey);

    print_separator("Chocolatey");

    let sudo = ctx.require_sudo()?;

    let mut command = sudo.execute(ctx, &choco)?;
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
        let sudo = ctx.require_sudo()?;
        sudo.execute(ctx, &winget)?
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

fn get_wsl_distributions(ctx: &ExecutionContext, wsl: &Path) -> Result<Vec<String>> {
    let output = ctx
        .execute(wsl)
        .always()
        .args(["--list", "-q"])
        .output_checked_utf8()?
        .stdout;
    Ok(output
        .lines()
        .map(|x| x.replace(['\u{0}', '\r'], "").trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect())
}

fn upgrade_wsl_distribution(wsl: &Path, dist: &str, ctx: &ExecutionContext) -> Result<()> {
    let topgrade = ctx
        .execute(wsl)
        .always()
        .args(["-d", dist, "bash", "-lc", "which topgrade"])
        .output_checked_utf8()
        .map_err(|_| SkipStep(t!("Could not find Topgrade installed in WSL").to_string()))?
        .stdout // The normal output from `which topgrade` appends a newline, so we trim it here.
        .trim_end()
        .to_owned();

    let mut command = ctx.execute(wsl);

    // WSL joins the arguments following the Linux command into a command line,
    // so arguments appended after the `bash -lc` script are not reliably passed
    // through as Bash positional parameters. Keep the complete script in one
    // argument and shell-quote the values that originated outside this function.
    let script = wsl_topgrade_script(dist, &topgrade, ctx.config().verbose(), ctx.config().yes(Step::Wsl));
    command.args(["-d", dist, "bash", "-lc", &script]);

    command.status_checked()
}

fn wsl_topgrade_script(dist: &str, topgrade: &str, verbose: bool, yes: bool) -> String {
    let mut script = format!(
        "TOPGRADE_PREFIX={} exec {}",
        shell_words::quote(dist),
        shell_words::quote(topgrade)
    );
    if verbose {
        script.push_str(" -v");
    }
    if yes {
        script.push_str(" -y");
    }
    script
}

pub fn run_wsl_topgrade(ctx: &ExecutionContext) -> Result<()> {
    if !is_wsl_installed()? {
        return Err(SkipStep(t!("WSL not installed").to_string()).into());
    }

    let wsl = require("wsl")?;
    let wsl_distributions = get_wsl_distributions(ctx, &wsl)?;
    let mut ran = false;
    let mut first_error = None;

    debug!("WSL distributions: {:?}", wsl_distributions);

    for distribution in wsl_distributions {
        let result = upgrade_wsl_distribution(&wsl, &distribution, ctx);
        debug!("Upgrading {:?}: {:?}", distribution, result);
        match result {
            Ok(()) => ran = true,
            Err(e) if e.is::<SkipStep>() => continue,
            Err(e) => {
                ran = true;
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }

    if let Some(error) = first_error {
        Err(error)
    } else if ran {
        Ok(())
    } else {
        Err(SkipStep(t!("Could not find Topgrade in any WSL distribution").to_string()).into())
    }
}

pub fn windows_update(ctx: &ExecutionContext) -> Result<()> {
    let powershell = ctx.require_powershell()?;

    print_separator(t!("Windows Update"));

    if !powershell.has_module(ctx, "PSWindowsUpdate") {
        print_warning(t!(
            "The PSWindowsUpdate PowerShell module isn't installed so Topgrade can't run Windows Update.\nInstall PSWindowsUpdate by running `Install-Module PSWindowsUpdate` in PowerShell."
        ));

        return Err(SkipStep(t!("PSWindowsUpdate is not installed").to_string()).into());
    }

    let mut cmd = "Import-Module PSWindowsUpdate; Install-WindowsUpdate -Verbose".to_string();

    if ctx.config().accept_all_windows_updates() {
        cmd.push_str(" -AcceptAll");
    }

    match ctx.config().windows_updates_auto_reboot() {
        UpdatesAutoReboot::Yes => cmd.push_str(" -AutoReboot"),
        UpdatesAutoReboot::No => cmd.push_str(" -IgnoreReboot"),
        UpdatesAutoReboot::Ask => (), // Prompting is the default for Install-WindowsUpdate
    }

    powershell.build_command(ctx, &cmd, true)?.status_checked()
}

pub fn microsoft_store(ctx: &ExecutionContext) -> Result<()> {
    let powershell = ctx.require_powershell()?;

    print_separator(t!("Microsoft Store"));

    println!("{}", t!("Scanning for updates..."));

    // Scan for updates using the MDM UpdateScanMethod
    // This method is also available for non-MDM devices
    let cmd = r#"(Get-CimInstance -Namespace "Root\cimv2\mdm\dmmap" -ClassName "MDM_EnterpriseModernAppManagement_AppManagement01" | Invoke-CimMethod -MethodName UpdateScanMethod).ReturnValue"#;

    powershell
        .build_command(ctx, cmd, true)?
        .output_checked_with_utf8(|output| {
            if !output.status.success() {
                return Err(());
            }
            let ret_val = output.stdout.trim();
            debug!("Command return value: {}", ret_val);
            if ret_val == "0" { Ok(()) } else { Err(()) }
        })?;
    println!(
        "{}",
        t!("Success, Microsoft Store apps are being updated in the background")
    );
    Ok(())
}

pub fn reboot(_ctx: &ExecutionContext) -> Result<()> {
    system_shutdown::reboot().map_err(Into::into)
}

pub fn shutdown(_ctx: &ExecutionContext) -> Result<()> {
    system_shutdown::shutdown().map_err(Into::into)
}

pub fn insert_startup_scripts(ctx: &ExecutionContext, git_repos: &mut RepoStep) -> Result<()> {
    let startup_dir = crate::WINDOWS_DIRS
        .data_dir()
        .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
    for entry in std::fs::read_dir(&startup_dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) == Some("lnk")
            && let Ok(lnk) = parselnk::Lnk::try_from(Path::new(&path))
        {
            debug!("Startup link: {:?}", lnk);
            if let Some(path) = lnk.relative_path() {
                git_repos.insert_if_repo(ctx, startup_dir.join(path));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::wsl_topgrade_script;

    #[test]
    fn wsl_topgrade_script_quotes_dynamic_values_and_forwards_flags() {
        assert_eq!(
            wsl_topgrade_script("Distro's name", "/home/user/top grade", true, true),
            "TOPGRADE_PREFIX='Distro'\\''s name' exec '/home/user/top grade' -v -y"
        );
    }
}
