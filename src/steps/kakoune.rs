use crate::error::TopgradeError;
use crate::terminal::print_separator;
use crate::utils::require;
use anyhow::Result;

use crate::execution_context::ExecutionContext;
use crate::executor::ExecutorOutput;

const UPGRADE_KAK: &str = include_str!("upgrade.kak");

pub fn upgrade_kak_plug(ctx: &ExecutionContext) -> Result<()> {
    let kak = require("kak")?;

    print_separator("Kakoune");

    let mut command = ctx.run_type().execute(kak);
    command.args(["-ui", "dummy", "-e", UPGRADE_KAK]);

    let output = command.output()?;

    if let ExecutorOutput::Wet(output) = output {
        let status = output.status;
        if !status.success() {
            return Err(TopgradeError::ProcessFailed(status).into());
        } else {
            println!("Plugins upgraded")
        }
    }

    Ok(())
}
