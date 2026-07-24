use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use color_eyre::eyre::Result;
use rust_i18n::t;
use std::fs;
use tracing::debug;

fn is_openbsd_current() -> Result<bool> {
    let motd_content = fs::read_to_string("/etc/motd")?;
    let is_current = ["-current", "-beta"].iter().any(|&s| motd_content.contains(s));

    debug!("OpenBSD is -current/-beta: {is_current}");

    Ok(is_current)
}

pub fn upgrade_openbsd(ctx: &ExecutionContext) -> Result<()> {
    print_separator(t!("OpenBSD Update"));

    let sudo = ctx.require_sudo()?;

    let is_current = is_openbsd_current()?;

    if is_current {
        sudo.execute(ctx, "/usr/sbin/sysupgrade")?.arg("-sn").status_checked()
    } else {
        sudo.execute(ctx, "/usr/sbin/syspatch")?.status_checked()
    }
}

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    print_separator(t!("OpenBSD Packages"));

    let sudo = ctx.require_sudo()?;

    let is_current = is_openbsd_current()?;

    if ctx.config().cleanup() {
        sudo.execute(ctx, "/usr/sbin/pkg_delete")?.arg("-ac").status_checked()?;
    }

    sudo.execute(ctx, "/usr/sbin/pkg_add")?
        .arg("-u")
        .arg_if(is_current, "-Dsnap")
        .status_checked()
}
