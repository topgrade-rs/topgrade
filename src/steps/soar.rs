use color_eyre::eyre::Result;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::require;

pub fn run_soar(ctx: &ExecutionContext) -> Result<()> {
    let soar = require("soar")?;

    print_separator("soar");

    ctx.execute(soar).arg("update").status_checked()
}
