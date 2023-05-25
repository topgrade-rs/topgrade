use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::sudo::Sudo;
use crate::terminal::print_separator;
use crate::utils::require_option;
use color_eyre::eyre::Result;
use std::process::Command;

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), String::from("No sudo detected"))?;
    print_separator("DragonFly BSD Packages");
    ctx.execute(sudo)
        .args(["/usr/local/sbin/pkg", "upgrade"])
        .status_checked()
}

pub fn audit_packages(sudo: Option<&Sudo>) -> Result<()> {
    if let Some(sudo) = sudo {
        println!();
        Command::new(sudo)
            .args(["/usr/local/sbin/pkg", "audit", "-Fr"])
            .status_checked()?;
    }
    Ok(())
}
