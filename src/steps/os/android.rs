use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::print_separator;
use crate::utils::require;
use crate::utils::which_one;
use color_eyre::Result;
use color_eyre::eyre::OptionExt;

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let pkg = which_one(["nala", "pkg"])?.ok_or_eyre("Expected `nala` or `pkg` to be available")?;

    print_separator("Termux Packages");

    let is_nala = pkg.ends_with("nala");

    ctx.execute(&pkg)
        .arg("upgrade")
        .arg_if(ctx.config().yes(Step::System), "-y")
        .status_checked()?;

    if !is_nala && ctx.config().cleanup() {
        ctx.execute(&pkg).arg("clean").status_checked()?;

        let apt = require("apt")?;

        ctx.execute(apt)
            .arg("autoremove")
            .arg_if(ctx.config().yes(Step::System), "-y")
            .status_checked()?;
    }

    Ok(())
}
