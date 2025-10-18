use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::print_separator;
use crate::utils;
use color_eyre::eyre::Result;

pub fn run_atuin(ctx: &ExecutionContext) -> Result<()> {
    // Check if this step is installed, if not, then this update will be skipped.
    let atuin = utils::require("atuin-update")?;

    // Print the separator
    print_separator("atuin-update");

    // Invoke the new step to get things updated!
    ctx.execute(atuin).status_checked()
}
