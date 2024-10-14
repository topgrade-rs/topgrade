use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::{get_require_sudo_string, require_option};
use color_eyre::eyre::Result;
use rust_i18n::t;

fn is_openbsd_current(ctx: &ExecutionContext) -> Result<bool> {
    if ctx.config().dry_run() {
        println!("Would check if OpenBSD is -current");
        Ok(false) // Default to false for dry-run
    } else {
        let output = ctx.run_type().execute("uname").arg("-r").output_checked()?;

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().ends_with("-current"))
    }
}

pub fn upgrade_openbsd(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("OpenBSD Update"));

    if ctx.config().dry_run() {
        println!("Would update the OpenBSD system");
        return Ok(());
    }

    let mut args = vec!["/usr/sbin/sysupgrade", "-n"];
    if is_openbsd_current(ctx)? {
        args.push("-s");
    }

    ctx.run_type().execute(sudo).args(&args).status_checked()
}

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("OpenBSD Packages"));

    if ctx.config().dry_run() {
        println!("Would update OpenBSD packages");
        return Ok(());
    }

    if ctx.config().cleanup() {
        ctx.run_type()
            .execute(sudo)
            .args(["/usr/sbin/pkg_delete", "-ac"])
            .status_checked()?;
    }

    let mut args = vec!["/usr/sbin/pkg_add", "-u"];
    if is_openbsd_current(ctx)? {
        args.push("-Dsnap");
    }

    ctx.run_type().execute(sudo).args(&args).status_checked()?;

    Ok(())
}
