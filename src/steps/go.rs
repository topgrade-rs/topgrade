use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::Result;

use crate::command::CommandExt;
use crate::executor::RunType;
use crate::terminal::print_separator;
use crate::utils;
use crate::utils::PathExt;

/// <https://github.com/Gelio/go-global-update>
pub fn run_go_global_update(run_type: RunType) -> Result<()> {
    let go_global_update = require_go_bin("go-global-update")?;

    print_separator("go-global-update");

    run_type.execute(go_global_update).status_checked()
}

/// <https://github.com/nao1215/gup>
pub fn run_go_gup(run_type: RunType) -> Result<()> {
    let gup = require_go_bin("gup")?;

    print_separator("gup");

    run_type.execute(gup).arg("update").status_checked()
}

/// Get the path of a Go binary.
fn require_go_bin(name: &str) -> Result<PathBuf> {
    let go = utils::require("go")?;
    let gopath_output = Command::new(go).args(["env", "GOPATH"]).output_checked_utf8()?;
    let gopath = gopath_output.stdout.trim();

    utils::require(name)
        .unwrap_or_else(|_| PathBuf::from(gopath).join("bin").join(name))
        .require()
}
