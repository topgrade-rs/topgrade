use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::t;
use crate::terminal::print_separator;
use crate::utils::{get_require_sudo_string, require_option};
use color_eyre::eyre::Result;
use std::process::Command;

fn is_openbsd_current() -> bool {
    let output = Command::new("uname")
        .arg("-r")
        .output()
        .expect("Failed to execute uname command");

    let version = String::from_utf8_lossy(&output.stdout);
    version.trim().ends_with("-current")
}

pub fn upgrade_openbsd(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("OpenBSD Update"));

    let mut args = vec!["/usr/sbin/sysupgrade", "-n"];
    if is_openbsd_current() {
        args.push("-s");
    }

    ctx.run_type().execute(sudo).args(&args).status_checked()
}

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("OpenBSD Packages"));

    if ctx.config().cleanup() {
        ctx.run_type()
            .execute(sudo)
            .args(["/usr/sbin/pkg_delete", "-ac"])
            .status_checked()?;
    }

    let mut args = vec!["/usr/sbin/pkg_add", "-u"];
    if is_openbsd_current() {
        args.push("-Dsnap");
    }

    ctx.run_type().execute(sudo).args(&args).status_checked()?;

    Ok(())
}
