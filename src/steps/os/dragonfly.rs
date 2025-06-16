use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::print_separator;
use crate::utils::{get_require_sudo_string, require_option};
use color_eyre::eyre::Result;
use std::process::Command;

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("DragonFly BSD Packages"));
    let mut cmd = ctx.run_type().execute(sudo);
    cmd.args(["/usr/local/sbin/pkg", "upgrade"]);
    if ctx.config().yes(Step::System) {
        cmd.arg("-y");
    }
    cmd.status_checked()
}

pub fn audit_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;

    print_separator(t!("DragonFly BSD Audit"));

    #[allow(clippy::disallowed_methods)]
    if !Command::new(sudo)
        .args(["/usr/local/sbin/pkg", "audit", "-Fr"])
        .status()?
        .success()
    {
        println!(t!(
            "The package audit was successful, but vulnerable packages still remain on the system"
        ));
    }
    Ok(())
}
