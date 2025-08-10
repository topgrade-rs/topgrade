use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::{print_separator, prompt_yesno};
use crate::utils::require;
use color_eyre::eyre::Result;
use rust_i18n::t;
use std::collections::HashSet;
use std::fs;
use std::process::Command;
use tracing::debug;

pub fn run_macports(ctx: &ExecutionContext) -> Result<()> {
    let sudo = ctx.require_sudo()?;
    let port = require("port")?;

    print_separator("MacPorts");
    sudo.execute(ctx, &port)?.arg("selfupdate").status_checked()?;
    sudo.execute(ctx, &port)?
        .args(["-u", "upgrade", "outdated"])
        .status_checked()?;
    if ctx.config().cleanup() {
        sudo.execute(ctx, &port)?.args(["-N", "reclaim"]).status_checked()?;
    }

    Ok(())
}

pub fn run_mas(ctx: &ExecutionContext) -> Result<()> {
    let mas = require("mas")?;
    print_separator(t!("macOS App Store"));

    ctx.execute(mas).arg("upgrade").status_checked()
}

pub fn upgrade_macos(ctx: &ExecutionContext) -> Result<()> {
    print_separator(t!("macOS system update"));

    let should_ask = !(ctx.config().yes(Step::System) || ctx.config().dry_run());
    if should_ask {
        println!("{}", t!("Finding available software"));
        if system_update_available()? {
            let answer = prompt_yesno(t!("A system update is available. Do you wish to install it?").as_ref())?;
            if !answer {
                return Ok(());
            }
            println!();
        } else {
            println!("{}", t!("No new software available."));
            return Ok(());
        }
    }

    let mut command = ctx.execute("softwareupdate");
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
            let mut command = ctx.execute(&sparkle);
            command.args(["bundle", "--check-immediately", "--application"]);
            command.arg(application.path());
            command.status_checked()?;
        }
    }
    Ok(())
}

pub fn update_xcodes(ctx: &ExecutionContext) -> Result<()> {
    let xcodes = require("xcodes")?;
    print_separator("Xcodes");

    let should_ask = !(ctx.config().yes(Step::Xcodes) || ctx.config().dry_run());

    let releases = ctx.execute(&xcodes).args(["update"]).output_checked_utf8()?.stdout;

    let releases_installed: Vec<String> = releases
        .lines()
        .filter(|r| r.contains("(Installed)"))
        .map(String::from)
        .collect();

    if releases_installed.is_empty() {
        println!("{}", t!("No Xcode releases installed."));
        return Ok(());
    }

    let (installed_gm, installed_beta, installed_regular) =
        releases_installed
            .iter()
            .fold((false, false, false), |(gm, beta, regular), release| {
                (
                    gm || release.contains("GM") || release.contains("Release Candidate"),
                    beta || release.contains("Beta"),
                    regular
                        || !(release.contains("GM")
                            || release.contains("Release Candidate")
                            || release.contains("Beta")),
                )
            });

    let releases_gm = releases
        .lines()
        .filter(|&r| r.matches("GM").count() > 0 || r.matches("Release Candidate").count() > 0)
        .map(String::from)
        .collect();
    let releases_beta = releases
        .lines()
        .filter(|&r| r.matches("Beta").count() > 0)
        .map(String::from)
        .collect();
    let releases_regular = releases
        .lines()
        .filter(|&r| {
            r.matches("GM").count() == 0
                && r.matches("Release Candidate").count() == 0
                && r.matches("Beta").count() == 0
        })
        .map(String::from)
        .collect();

    if installed_gm {
        process_xcodes_releases(releases_gm, should_ask, ctx)?;
    }
    if installed_beta {
        process_xcodes_releases(releases_beta, should_ask, ctx)?;
    }
    if installed_regular {
        process_xcodes_releases(releases_regular, should_ask, ctx)?;
    }

    let releases_new = ctx.execute(&xcodes).args(["list"]).output_checked_utf8()?.stdout;

    let releases_gm_new_installed: HashSet<_> = releases_new
        .lines()
        .filter(|release| {
            release.contains("(Installed)") && (release.contains("GM") || release.contains("Release Candidate"))
        })
        .collect();
    let releases_beta_new_installed: HashSet<_> = releases_new
        .lines()
        .filter(|release| release.contains("(Installed)") && release.contains("Beta"))
        .collect();
    let releases_regular_new_installed: HashSet<_> = releases_new
        .lines()
        .filter(|release| {
            release.contains("(Installed)")
                && !(release.contains("GM") || release.contains("Release Candidate") || release.contains("Beta"))
        })
        .collect();

    for releases_new_installed in [
        releases_gm_new_installed,
        releases_beta_new_installed,
        releases_regular_new_installed,
    ] {
        if should_ask && releases_new_installed.len() == 2 {
            let answer_uninstall =
                prompt_yesno(t!("Would you like to move the former Xcode release to the trash?").as_ref())?;
            if answer_uninstall {
                let _ = ctx
                    .execute(&xcodes)
                    .args([
                        "uninstall",
                        releases_new_installed.iter().next().copied().unwrap_or_default(),
                    ])
                    .status_checked();
            }
        }
    }

    Ok(())
}

pub fn process_xcodes_releases(releases_filtered: Vec<String>, should_ask: bool, ctx: &ExecutionContext) -> Result<()> {
    let xcodes = require("xcodes")?;

    if releases_filtered.last().map_or(true, |s| !s.contains("(Installed)")) && !releases_filtered.is_empty() {
        println!(
            "{} {}",
            t!("New Xcode release detected:"),
            releases_filtered.last().cloned().unwrap_or_default()
        );
        if should_ask {
            let answer_install = prompt_yesno(t!("Would you like to install it?").as_ref())?;
            if answer_install {
                let _ = ctx
                    .execute(xcodes)
                    .args(["install", &releases_filtered.last().cloned().unwrap_or_default()])
                    .status_checked();
            }
            println!();
        }
    }

    Ok(())
}
