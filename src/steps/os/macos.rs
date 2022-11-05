use crate::execution_context::ExecutionContext;
use crate::executor::{CommandExt, RunType};
use crate::terminal::{print_separator, prompt_yesno};
use crate::{error::TopgradeError, utils::require, Step};
use anyhow::Result;
use log::debug;
use std::fs;
use std::process::Command;

pub fn run_macports(ctx: &ExecutionContext) -> Result<()> {
    require("port")?;
    let sudo = ctx.sudo().as_ref().unwrap();
    print_separator("MacPorts");
    ctx.run_type().execute(sudo).args(["port", "selfupdate"]).check_run()?;
    ctx.run_type()
        .execute(sudo)
        .args(["port", "-u", "upgrade", "outdated"])
        .check_run()?;
    if ctx.config().cleanup() {
        ctx.run_type()
            .execute(sudo)
            .args(["port", "-N", "reclaim"])
            .check_run()?;
    }

    Ok(())
}

pub fn run_mas(run_type: RunType) -> Result<()> {
    let mas = require("mas")?;
    print_separator("macOS App Store");

    run_type.execute(mas).arg("upgrade").check_run()
}

pub fn upgrade_macos(ctx: &ExecutionContext) -> Result<()> {
    print_separator("macOS system update");

    let should_ask = !(ctx.config().yes(Step::System)) || (ctx.config().dry_run());
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

    command.check_run()
}

fn system_update_available() -> Result<bool> {
    let output = Command::new("softwareupdate").arg("--list").output()?;
    debug!("{:?}", output);

    let status = output.status;
    if !status.success() {
        return Err(TopgradeError::ProcessFailed(status).into());
    }
    let string_output = String::from_utf8(output.stderr)?;
    debug!("{:?}", string_output);
    Ok(!string_output.contains("No new software available"))
}

pub fn run_sparkle(ctx: &ExecutionContext) -> Result<()> {
    let sparkle = require("sparkle")?;

    print_separator("Sparkle");

    for application in (fs::read_dir("/Applications")?).flatten() {
        let probe = Command::new(&sparkle)
            .args(["--probe", "--application"])
            .arg(application.path())
            .check_output();
        if probe.is_ok() {
            let mut command = ctx.run_type().execute(&sparkle);
            command.args(["bundle", "--check-immediately", "--application"]);
            command.arg(application.path());
            command.spawn()?.wait()?;
        }
    }
    Ok(())
}
