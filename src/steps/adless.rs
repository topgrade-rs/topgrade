use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::require;
use color_eyre::eyre::Result;

/// Update Adless hosts list using `sudo adless update`
pub fn run_adless(ctx: &ExecutionContext) -> Result<()> {
    let adless = require("adless")?;

    print_separator("Adless");

    let sudo = ctx.require_sudo()?;
    sudo.execute(ctx, &adless)?.arg("update").status_checked()
}
