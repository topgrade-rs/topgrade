use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::require_option;
use color_eyre::eyre::Result;
use std::path::PathBuf;

pub fn upgrade_openbsd(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), String::from("No sudo detected"))?;
    print_separator("OpenBSD Update");
    ctx.run_type()
        .execute(sudo)
        .args(&["/usr/sbin/sysupgrade", "-n"])
        .status_checked()
}

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), String::from("No sudo detected"))?;
    print_separator("OpenBSD Packages");
    ctx.run_type()
        .execute(sudo)
        .args(&["/usr/sbin/pkg_add", "-u"])
        .status_checked()
}
