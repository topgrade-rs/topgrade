use crate::terminal::print_separator;
use crate::utils::require;
use color_eyre::eyre::Result;
use rust_i18n::t;

use crate::execution_context::ExecutionContext;

const UPGRADE_KAK: &str = include_str!("upgrade.kak");

pub fn upgrade_kak_plug(ctx: &ExecutionContext) -> Result<()> {
    let kak = require("kak")?;

    print_separator("Kakoune");

    // TODO: Why suppress output for this command?
    ctx.run_type()
        .execute(kak)
        .args(["-ui", "dummy", "-e", UPGRADE_KAK])
        .output()?;

    println!("{}", t!("Plugins upgraded"));

    Ok(())
}
