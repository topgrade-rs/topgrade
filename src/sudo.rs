use std::path::PathBuf;

use color_eyre::eyre::Context;
use color_eyre::eyre::Result;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::which;

/// Get the path of the `sudo` utility.
///
/// Detects `doas`, `sudo`, `gsudo`, or `pkexec`.
pub fn path() -> Option<PathBuf> {
    which("doas")
        .or_else(|| which("sudo"))
        .or_else(|| which("gsudo"))
        .or_else(|| which("pkexec"))
}

/// Elevate permissions with `sudo`.
pub fn elevate(ctx: &ExecutionContext, sudo: Option<&PathBuf>) -> Result<()> {
    if let Some(sudo) = sudo {
        print_separator("Sudo");
        ctx.run_type()
            .execute(sudo)
            // TODO: Does this work with `doas`, `pkexec`, `gsudo`, GNU `sudo`...?
            .arg("-v")
            .status_checked()
            .wrap_err("Failed to elevate permissions")?;
    }

    Ok(())
}
