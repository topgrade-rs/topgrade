use color_eyre::eyre::Result;

use crate::command::CommandExt;
use crate::error::SkipStep;
use crate::step::Step;
use crate::terminal::print_separator;
use crate::{execution_context::ExecutionContext, utils::require};
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;

fn list_toolboxes(ctx: &ExecutionContext, toolbx: &Path) -> Result<Vec<String>> {
    let output = ctx
        .execute(toolbx)
        .always()
        .args(["list", "--containers"])
        .output_checked_with_utf8(|output| {
            if output.status.success() || is_opensuse_toolbox_error(&output.stderr) {
                Ok(())
            } else {
                Err(())
            }
        })?;

    if is_opensuse_toolbox_error(&output.stderr) {
        return Err(SkipStep("Command `toolbox` is openSUSE Toolbox, not Toolbx".to_string()).into());
    }

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

fn is_opensuse_toolbox_error(stderr: &str) -> bool {
    stderr.contains("unrecognized option '--containers'")
        || stderr.contains("unrecognized option '--container'")
        || stderr.contains("unrecognized option: containers")
}

pub fn run_toolbx(ctx: &ExecutionContext) -> Result<()> {
    let toolbx = require("toolbox")?;
    let toolboxes = list_toolboxes(ctx, &toolbx)?;

    print_separator("Toolbx");
    debug!("Toolboxes to inspect: {:?}", toolboxes);

    let mut topgrade_path = PathBuf::from("/run/host");
    // Path of the running Topgrade executable
    // Skip 1 to eliminate the path root, otherwise push overwrites the path
    topgrade_path.push(std::env::current_exe()?.components().skip(1).collect::<PathBuf>());
    let topgrade_path = topgrade_path.to_str().unwrap();

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
    use super::is_opensuse_toolbox_error;

    #[test]
    fn detects_opensuse_toolbox_unrecognized_containers_option() {
        assert!(is_opensuse_toolbox_error("toolbox: unrecognized option '--containers'"));
    }

    #[test]
    fn does_not_treat_other_errors_as_opensuse_toolbox() {
        assert!(!is_opensuse_toolbox_error("toolbox: failed to list containers"));
    }
}
