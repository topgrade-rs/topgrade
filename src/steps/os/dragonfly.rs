use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::{require_option, REQUIRE_SUDO};
use color_eyre::eyre::Result;
use std::process::Command;

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), REQUIRE_SUDO.to_string())?;
    print_separator("DragonFly BSD Packages");
    ctx.execute(sudo)
        .args(["/usr/local/sbin/pkg", "upgrade"])
        .arg(if ctx.config().yes(Step::System) { "-y" } else { "" })
        .status_checked()
}

pub fn audit_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), REQUIRE_SUDO.to_string())?;
    println!();
    Command::new(sudo)
        .args(["/usr/local/sbin/pkg", "audit", "-Fr"])
        .status_checked()?;
    Ok(())
}
