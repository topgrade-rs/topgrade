use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::{print_separator, prompt_yesno};
use crate::utils::{require_option, PathExt, REQUIRE_SUDO};
use crate::{utils::require, Step};
use color_eyre::eyre::Result;
use std::fs;
use std::process::Command;
use tracing::debug;

pub fn run_macports(ctx: &ExecutionContext) -> Result<()> {
    require("port")?;
    let sudo = require_option(ctx.sudo().as_ref(), REQUIRE_SUDO.to_string())?;

    print_separator("MacPorts");
    ctx.run_type()
        .execute(sudo)
        .args(["port", "selfupdate"])
        .status_checked()?;
    ctx.run_type()
        .execute(sudo)
        .args(["port", "-u", "upgrade", "outdated"])
        .status_checked()?;
    if ctx.config().cleanup() {
        ctx.run_type()
            .execute(sudo)
            .args(["port", "-N", "reclaim"])
            .status_checked()?;
    }

    Ok(())
}

pub fn run_mas(ctx: &ExecutionContext) -> Result<()> {
    let mas = require("mas")?;
    print_separator("macOS App Store");

    ctx.run_type().execute(mas).arg("upgrade").status_checked()
}

pub fn upgrade_macos(ctx: &ExecutionContext) -> Result<()> {
    print_separator("macOS system update");

    let should_ask = !(ctx.config().yes(Step::System) || ctx.config().dry_run());
    if should_ask {
        println!("Finding available software");
        if system_update_available()? {
            let answer = prompt_yesno("A system update is available. Do you wish to install it?")?;
            if !answer {
                return Ok(());
            }
            println!();
        } else {
            println!("No new software available.");
            return Ok(());
        }
    }

    let mut command = ctx.run_type().execute("softwareupdate");
    command.args(["--install", "--all"]);

    if should_ask {
        command.arg("--no-scan");
    }

    command.status_checked()
}

fn system_update_available() -> Result<bool> {
    let output = Command::new("softwareupdate").arg("--list").output_checked_utf8()?;

    debug!("{:?}", output);

    Ok(!output.stderr.contains("No new software available"))
}

pub fn run_sparkle(ctx: &ExecutionContext) -> Result<()> {
    let sparkle = require("sparkle")?;

    print_separator("Sparkle");

    for application in (fs::read_dir("/Applications")?).flatten() {
        let probe = Command::new(&sparkle)
            .args(["--probe", "--application"])
            .arg(application.path())
            .output_checked_utf8();
        if probe.is_ok() {
            let mut command = ctx.run_type().execute(&sparkle);
            command.args(["bundle", "--check-immediately", "--application"]);
            command.arg(application.path());
            command.status_checked()?;
        }
    }
    Ok(())
}

pub fn run_msupdate(ctx: &ExecutionContext) -> Result<()> {
    // Per the documentation
    //
    // https://learn.microsoft.com/en-us/deployoffice/mac/update-office-for-mac-using-msupdate
    //
    // The binary `msupate` is located at:
    // `/Library/Application\ Support/Microsoft/MAU2.0/Microsoft\ AutoUpdate.app/Contents/MacOS/msupdate`
    // and its parent directory is not in `$PATH`

    let msupdate = r#"/Library/Application Support/Microsoft/MAU2.0/Microsoft AutoUpdate.app/Contents/MacOS/msupdate"#
        .require()?;

    print_separator("msupdate");

    // Download and install all available updates: ./msupdate --install
    ctx.run_type().execute(msupdate).arg("--install").status_checked()
}
