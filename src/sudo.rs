use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use serde::Deserialize;
use strum::AsRefStr;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::executor::Executor;
use crate::terminal::print_separator;
use crate::utils::which;

#[derive(Clone, Debug)]
pub struct Sudo {
    /// The path to the `sudo` binary.
    path: PathBuf,
    /// The type of program being used as `sudo`.
    kind: SudoKind,
}

impl Sudo {
    /// Get the `sudo` binary for this platform.
    pub fn detect() -> Option<Self> {
        which("doas")
            .map(|p| (p, SudoKind::Doas))
            .or_else(|| which("please").map(|p| (p, SudoKind::Please)))
            .or_else(|| which("sudo").map(|p| (p, SudoKind::Sudo)))
            .or_else(|| which("gsudo").map(|p| (p, SudoKind::Gsudo)))
            .or_else(|| which("pkexec").map(|p| (p, SudoKind::Pkexec)))
            .map(|(path, kind)| Self { path, kind })
    }

    /// Create Sudo from SudoKind, if found in the system
    pub fn new(kind: SudoKind) -> Option<Self> {
        which(kind.as_ref()).map(|path| Self { path, kind })
    }

    /// Elevate permissions with `sudo`.
    ///
    /// This helps prevent blocking `sudo` prompts from stopping the run in the middle of a
    /// step.
    ///
    /// See: https://github.com/topgrade-rs/topgrade/issues/205
    pub fn elevate(&self, ctx: &ExecutionContext) -> Result<()> {
        print_separator("Sudo");
        let mut cmd = ctx.run_type().execute(self);
        match self.kind {
            SudoKind::Doas => {
                // `doas` doesn't have anything like `sudo -v` to cache credentials,
                // so we just execute a dummy `echo` command so we have something
                // unobtrusive to run.
                // See: https://man.openbsd.org/doas
                cmd.arg("echo");
            }
            SudoKind::Please => {
                // From `man please`
                //   -w, --warm
                //   Warm the access token and exit.
                cmd.arg("-w");
            }
            SudoKind::Sudo => {
                // From `man sudo` on macOS:
                //   -v, --validate
                //   Update the user's cached credentials, authenticating the user
                //   if necessary.  For the sudoers plugin, this extends the sudo
                //   timeout for another 5 minutes by default, but does not run a
                //   command.  Not all security policies support cached credentials.
                cmd.arg("-v");
            }
            SudoKind::Gsudo => {
                // Shows current user, cache and console status.
                // See: https://gerardog.github.io/gsudo/docs/usage
                cmd.arg("status");
            }
            SudoKind::Pkexec => {
                // I don't think this does anything; `pkexec` usually asks for
                // authentication every time, although it can be configured
                // differently.
                //
                // See the note for `doas` above.
                //
                // See: https://linux.die.net/man/1/pkexec
                cmd.arg("echo");
            }
        }
        cmd.status_checked().wrap_err("Failed to elevate permissions")
    }

    /// Execute a command with `sudo`.
    pub fn execute_elevated(&self, ctx: &ExecutionContext, command: &Path, interactive: bool) -> Executor {
        let mut cmd = ctx.run_type().execute(self);

        if let SudoKind::Sudo = self.kind {
            cmd.arg("--preserve-env=DIFFPROG");
        }

        if interactive {
            cmd.arg("-i");
        }

        cmd.arg(command);

        cmd
    }
}

#[derive(Clone, Copy, Debug, Deserialize, AsRefStr)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SudoKind {
    Doas,
    Please,
    Sudo,
    Gsudo,
    Pkexec,
}

impl AsRef<OsStr> for Sudo {
    fn as_ref(&self) -> &OsStr {
        self.path.as_ref()
    }
}
