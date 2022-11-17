use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::executor::RunType;
use crate::terminal::print_separator;
use crate::utils::require_option;
use crate::Step;
use color_eyre::eyre::Result;
use std::path::PathBuf;
use std::process::Command;

pub fn upgrade_freebsd(sudo: Option<&PathBuf>, run_type: RunType) -> Result<()> {
    let sudo = require_option(sudo, String::from("No sudo detected"))?;
    print_separator("FreeBSD Update");
    run_type
        .execute(sudo)
        .args(["/usr/sbin/freebsd-update", "fetch", "install"])
        .status_checked()
}

pub fn upgrade_packages(ctx: &ExecutionContext, sudo: Option<&PathBuf>, run_type: RunType) -> Result<()> {
    let sudo = require_option(sudo, String::from("No sudo detected"))?;
    print_separator("FreeBSD Packages");

    let mut command = run_type.execute(sudo);

    command.args(["/usr/sbin/pkg", "upgrade"]);
    if ctx.config().yes(Step::System) {
        command.arg("-y");
    }
    command.check_run()
}

pub fn audit_packages(sudo: &Option<PathBuf>) -> Result<()> {
    if let Some(sudo) = sudo {
        println!();
        Command::new(sudo)
            .args(["/usr/sbin/pkg", "audit", "-Fr"])
            .status_checked()?;
    }
    Ok(())
}
