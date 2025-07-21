use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::print_separator;
use crate::utils::require;
use crate::utils::which;
use color_eyre::Result;

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    //let pkg = require("pkg")?;
    let pkg = which("nala").or_else(|| which("pkg")).unwrap();

    print_separator("Termux Packages");

    let is_nala = pkg.ends_with("nala");

    let mut command = ctx.execute(&pkg);
    command.arg("upgrade");

    if ctx.config().yes(Step::System) {
        command.arg("-y");
    }
    command.status_checked()?;

    if !is_nala && ctx.config().cleanup() {
        ctx.execute(&pkg).arg("clean").status_checked()?;

        let apt = require("apt")?;
        let mut command = ctx.execute(apt);
        command.arg("autoremove");
        if ctx.config().yes(Step::System) {
            command.arg("-y");
        }
        command.status_checked()?;
    }

    Ok(())
}
