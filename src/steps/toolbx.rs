use color_eyre::eyre::{OptionExt, Result};

use crate::command::CommandExt;
use crate::error::SkipStep;
use crate::step::Step;
use crate::terminal::print_separator;
use crate::{execution_context::ExecutionContext, utils::require};
use rust_i18n::t;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;

enum Toolbox {
    Toolbx(PathBuf),
    OpenSuseToolbox,
}

impl Toolbox {
    fn toolbx(self) -> Result<PathBuf> {
        match self {
            Toolbox::Toolbx(toolbx) => Ok(toolbx),
            Toolbox::OpenSuseToolbox => {
                Err(SkipStep(t!("Command `toolbox` probably points to openSUSE's toolbox").to_string()).into())
            }
        }
    }
}

fn is_shell_script(path: &Path) -> Result<bool> {
    let mut magic = [0u8; 2];
    let read = File::open(path)?.read(&mut magic)?;
    Ok(read == 2 && &magic == b"#!")
}

fn get_toolbox() -> Result<Toolbox> {
    let toolbox = require("toolbox")?;

    if is_shell_script(&toolbox)? {
        debug!("Detected `toolbox` as openSUSE's toolbox");
        Ok(Toolbox::OpenSuseToolbox)
    } else {
        debug!("Detected `toolbox` as Toolbx");
        Ok(Toolbox::Toolbx(toolbox))
    }
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
    let toolbx = get_toolbox()?.toolbx()?;

    print_separator("Toolbx");
    let toolboxes = list_toolboxes(ctx, &toolbx)?;
    debug!("Toolboxes to inspect: {:?}", toolboxes);

    let mut topgrade_path = PathBuf::from("/run/host");
    // Path of the running Topgrade executable
    // Skip 1 to eliminate the path root, otherwise push overwrites the path
    topgrade_path.push(std::env::current_exe()?.components().skip(1).collect::<PathBuf>());
    let topgrade_path = topgrade_path.to_str().ok_or_eyre("Non-UTF-8 path")?;

    for tb in toolboxes.iter() {
        let topgrade_prefix = format!("TOPGRADE_PREFIX=Toolbx {tb}");
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
    use super::is_shell_script;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_shell_script() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("script");
        let binary = dir.path().join("binary");
        let empty = dir.path().join("empty");
        fs::write(&script, "#!/bin/bash\necho hi\n").unwrap();
        fs::write(&binary, b"\x7fELF\x02\x01\x01").unwrap();
        fs::write(&empty, "").unwrap();

        assert!(is_shell_script(&script).unwrap());
        assert!(!is_shell_script(&binary).unwrap());
        assert!(!is_shell_script(&empty).unwrap());
    }
}
