use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::Result;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils;
use crate::utils::PathExt;

/// <https://github.com/Gelio/go-global-update>
pub fn run_go_global_update(ctx: &ExecutionContext) -> Result<()> {
    let go_global_update = require_go_bin("go-global-update")?;

    print_separator("go-global-update");

    ctx.run_type().execute(go_global_update).status_checked()
}

/// <https://github.com/nao1215/gup>
pub fn run_go_gup(ctx: &ExecutionContext) -> Result<()> {
    let gup = require_go_bin("gup")?;

    print_separator("gup");

    ctx.run_type().execute(gup).arg("update").status_checked()
}

/// Get the path of a Go binary.
fn require_go_bin(name: &str) -> Result<PathBuf> {
    utils::require(name).or_else(|_| {
        let go = utils::require("go")?;
        // TODO: Does this work? `go help gopath` says that:
        // > The GOPATH environment variable lists places to look for Go code.
        // > On Unix, the value is a colon-separated string.
        // > On Windows, the value is a semicolon-separated string.
        // > On Plan 9, the value is a list.
        // Should we also fallback to the env variable?
        let gopath_output = Command::new(go).args(["env", "GOPATH"]).output_checked_utf8()?;
        let gopath = gopath_output.stdout.trim();

        PathBuf::from(gopath).join("bin").join(name).require()
    })
}
