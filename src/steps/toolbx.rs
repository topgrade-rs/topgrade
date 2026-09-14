use color_eyre::eyre::{OptionExt, Result};

use crate::command::CommandExt;
use crate::error::SkipStep;
use crate::step::Step;
use crate::terminal::print_separator;
use crate::{execution_context::ExecutionContext, utils::require};
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;

/// Locate the containers `toolbx` binary (<https://github.com/containers/toolbox>).
///
/// Several distributions ship an unrelated tool that is also called `toolbox`,
/// most notably openSUSE (<https://github.com/openSUSE/microos-toolbox>), so
/// finding a binary named `toolbox` in `PATH` is not enough. We confirm it is
/// the real containers `toolbx` by checking that `toolbox --version` reports
/// `toolbox version ...`, which the openSUSE tool does not.
fn require_containers_toolbx(ctx: &ExecutionContext) -> Result<PathBuf> {
    let toolbx = require("toolbox")?;

    let version_output = ctx.execute(&toolbx).always().arg("--version").output_checked_utf8();

    match version_output {
        Ok(output) if is_containers_toolbx_version(&output.stdout) => Ok(toolbx),
        _ => Err(SkipStep(
            "Found a `toolbox` binary, but it is not the containers toolbx (e.g. openSUSE's toolbox)".to_string(),
        )
        .into()),
    }
}

/// Whether `toolbox --version` output belongs to the containers `toolbx`.
///
/// containers `toolbx` prints `toolbox version <x.y.z>`, whereas openSUSE's
/// unrelated `toolbox` does not support `--version` at all.
fn is_containers_toolbx_version(stdout: &str) -> bool {
    stdout.trim_start().starts_with("toolbox version")
}

fn list_toolboxes(ctx: &ExecutionContext, toolbx: &Path) -> Result<Vec<String>> {
    let output = ctx
        .execute(toolbx)
        .always()
        .args(["list", "--containers"])
        .output_checked_utf8()?;

    let proc: Vec<String> = output
        .stdout
        .lines()
        // Skip the first line since that contains only status information
        .skip(1)
        .map(|line| match line.split_whitespace().nth(1) {
            Some(word) => word.to_string(),
            None => String::from(""),
        })
        .filter(|x| !x.is_empty())
        .collect();

    Ok(proc)
}

pub fn run_toolbx(ctx: &ExecutionContext) -> Result<()> {
    let toolbx = require_containers_toolbx(ctx)?;

    print_separator("Toolbx");
    let toolboxes = list_toolboxes(ctx, &toolbx)?;
    debug!("Toolboxes to inspect: {:?}", toolboxes);

    let mut topgrade_path = PathBuf::from("/run/host");
    // Path of the running Topgrade executable
    // Skip 1 to eliminate the path root, otherwise push overwrites the path
    topgrade_path.push(std::env::current_exe()?.components().skip(1).collect::<PathBuf>());
    let topgrade_path = topgrade_path.to_str().ok_or_eyre("Non-UTF-8 path")?;

    for tb in toolboxes.iter() {
        let topgrade_prefix = format!("TOPGRADE_PREFIX='Toolbx {tb}'");
        let mut args = vec![
            "run",
            "-c",
            tb,
            "env",
            &topgrade_prefix,
            topgrade_path,
            "--only",
            "system",
            "--no-self-update",
            "--notify-end",
            "never",
        ];
        if ctx.config().yes(Step::Toolbx) {
            args.push("--yes");
        }

        ctx.execute(&toolbx).args(&args).status_checked()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_containers_toolbx_version;

    #[test]
    fn recognizes_containers_toolbx() {
        assert!(is_containers_toolbx_version("toolbox version 0.0.99.3\n"));
        assert!(is_containers_toolbx_version("toolbox version 0.1.2"));
    }

    #[test]
    fn rejects_opensuse_toolbox() {
        // openSUSE's toolbox has no `--version`; it prints usage/errors instead.
        assert!(!is_containers_toolbx_version("Usage: toolbox [ options ] ...\n"));
        assert!(!is_containers_toolbx_version(""));
        assert!(!is_containers_toolbx_version("Unknown option: --version"));
    }
}
