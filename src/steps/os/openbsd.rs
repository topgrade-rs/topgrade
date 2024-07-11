use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::{get_require_sudo_string, require_option};
use color_eyre::eyre::Result;

pub fn upgrade_openbsd(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("OpenBSD Update"));
    ctx.run_type()
        .execute(sudo)
        .args(["/usr/sbin/sysupgrade", "-n"])
        .status_checked()
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

    ctx.run_type()
        .execute(sudo)
        .args(["/usr/sbin/pkg_add", "-u"])
        .status_checked()?;

    Ok(())
}
