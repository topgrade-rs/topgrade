use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::{get_require_sudo_string, require_option};
use color_eyre::eyre::Result;
use rust_i18n::t;
use std::fs;

fn is_openbsd_current(ctx: &ExecutionContext) -> Result<bool> {
    let motd_content = fs::read_to_string("/etc/motd")?;
    let is_current = motd_content.contains("-current");
    if ctx.config().dry_run() {
        println!("Would check if OpenBSD is -current");
        Ok(is_current)
    } else {
        Ok(is_current)
    }
}

pub fn upgrade_openbsd(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("OpenBSD Update"));

    let is_current = is_openbsd_current(ctx)?;

    if ctx.config().dry_run() {
        println!("Would upgrade the OpenBSD system");
        return Ok(());
    }

    let mut args = vec!["/usr/sbin/sysupgrade", "-n"];
    if is_current {
        args.push("-s");
    }

    ctx.run_type().execute(sudo).args(&args).status_checked()
}

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("OpenBSD Packages"));

    let is_current = is_openbsd_current(ctx)?;

    if ctx.config().dry_run() {
        println!("Would upgrade OpenBSD packages");
        return Ok(());
    }

    if ctx.config().cleanup() {
        ctx.run_type()
            .execute(sudo)
            .args(["/usr/sbin/pkg_delete", "-ac"])
            .status_checked()?;
    }

    let mut args = vec!["/usr/sbin/pkg_add", "-u"];
    if is_current {
        args.push("-Dsnap");
    }

    ctx.run_type().execute(sudo).args(&args).status_checked()?;

    Ok(())
}
