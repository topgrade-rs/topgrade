use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::require;
use crate::utils::which;
use crate::Step;
use anyhow::Result;

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    //let pkg = require("pkg")?;
    let pkg = which("nala").or_else(|| which("pkg")).unwrap();

    print_separator("Termux Packages");

    let is_nala = pkg.ends_with("nala");

    let mut command = ctx.run_type().execute(&pkg);
    command.arg("upgrade");

    if ctx.config().yes(Step::System) {
        command.arg("-y");
    }
    command.check_run()?;

    if !is_nala {
        if ctx.config().cleanup() {
            ctx.run_type().execute(&pkg).arg("clean").check_run()?;

            let apt = require("apt")?;
            let mut command = ctx.run_type().execute(&apt);
            command.arg("autoremove");
            if ctx.config().yes(Step::System) {
                command.arg("-y");
            }
            command.check_run()?;
        }
    }

    Ok(())
}
