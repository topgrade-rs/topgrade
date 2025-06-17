use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::print_separator;
use crate::utils::{get_require_sudo_string, require_option};
use color_eyre::Result;
use rust_i18n::t;
use std::process::Command;

pub fn upgrade_freebsd(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("FreeBSD Update"));
    ctx.run_type()
        .execute(sudo)
        .args(["/usr/sbin/freebsd-update", "fetch", "install"])
        .status_checked()
}

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("FreeBSD Packages"));

    let mut command = ctx.run_type().execute(sudo);

    command.args(["/usr/sbin/pkg", "upgrade"]);
    if ctx.config().yes(Step::System) {
        command.arg("-y");
    }
    command.status_checked()
}

pub fn audit_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;

    print_separator(t!("FreeBSD Audit"));

    Command::new(sudo)
        .args(["/usr/sbin/pkg", "audit", "-Fr"])
        .status_checked()?;
    Ok(())
}
