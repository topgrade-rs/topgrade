use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use serde::Deserialize;
use strum::AsRefStr;
use strum::Display;

use crate::command::CommandExt;
use crate::error::UnsupportedSudo;
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

#[derive(Clone, Debug, Default)]
/// Generic sudo options, translated into flags to pass to `sudo`. Depending on the sudo kind, OS
/// and system config, some options might be specified by default or unsupported.
pub struct SudoExecuteOpts<'a> {
    /// Run the command "interactively", i.e. inside a login shell.
    pub interactive: bool,
    /// Preserve environment variables across the sudo call. If an empty list is given, preserves
    /// all existing environment variables.
    pub preserve_env: Option<&'a [&'a str]>,
    /// Set the HOME environment variable to the target user's home directory.
    pub set_home: bool,
    /// Run the command as a user other than the root user.
    pub user: Option<&'a str>,
}

impl Sudo {
    /// Get the `sudo` binary or the `gsudo` binary in the case of `gsudo`
    /// masquerading as the `sudo` binary.
    fn determine_sudo_variant(sudo_p: PathBuf) -> (PathBuf, SudoKind) {
        match which("gsudo") {
            Some(gsudo_p) => {
                match std::fs::canonicalize(&gsudo_p).unwrap() == std::fs::canonicalize(&sudo_p).unwrap() {
                    true => (gsudo_p, SudoKind::Gsudo),
                    false => (sudo_p, SudoKind::Sudo),
                }
            }
            None => (sudo_p, SudoKind::Sudo),
        }
    }

    /// Get the `sudo` binary for this platform.
    pub fn detect() -> Option<Self> {
        which("doas")
            .map(|p| (p, SudoKind::Doas))
            .or_else(|| which("sudo").map(Self::determine_sudo_variant))
            .or_else(|| which("gsudo").map(|p| (p, SudoKind::Gsudo)))
            .or_else(|| which("pkexec").map(|p| (p, SudoKind::Pkexec)))
            .or_else(|| which("run0").map(|p| (p, SudoKind::Run0)))
            .or_else(|| which("please").map(|p| (p, SudoKind::Please)))
            .map(|(path, kind)| Self { path, kind })
    }

    /// Create Sudo from SudoKind, if found in the system
    pub fn new(kind: SudoKind) -> Option<Self> {
        which(kind.as_ref()).map(|path| Self { path, kind })
    }

    /// Gets the path to the `sudo` binary. Do not use this to execute `sudo` directly - either use
    /// [`Sudo::elevate`], or if you need to specify arguments to `sudo`, use [`Sudo::elevate_opts`].
    /// This way, sudo options can be specified generically and the actual arguments customized
    /// depending on the sudo kind.
    #[allow(unused)]
    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    /// Elevate permissions with `sudo`.
    ///
    /// This helps prevent blocking `sudo` prompts from stopping the run in the middle of a
    /// step.
    ///
    /// See: https://github.com/topgrade-rs/topgrade/issues/205
    pub fn elevate(&self, ctx: &ExecutionContext) -> Result<()> {
        print_separator("Sudo");
        let mut cmd = ctx.execute(&self.path);
        match self.kind {
            SudoKind::Doas => {
                // `doas` doesn't have anything like `sudo -v` to cache credentials,
                // so we just execute a dummy `echo` command so we have something
                // unobtrusive to run.
                // See: https://man.openbsd.org/doas
                cmd.arg("echo");
            }
            SudoKind::Sudo if cfg!(not(target_os = "windows")) => {
                // From `man sudo` on macOS:
                //   -v, --validate
                //   Update the user's cached credentials, authenticating the user
                //   if necessary.  For the sudoers plugin, this extends the sudo
                //   timeout for another 5 minutes by default, but does not run a
                //   command.  Not all security policies support cached credentials.
                cmd.arg("-v");
            }
            SudoKind::Sudo => {
                // Windows `sudo` doesn't cache credentials, so we just execute a
                // dummy command - the easiest on Windows is `rem` in cmd.
                // See: https://learn.microsoft.com/en-us/windows/advanced-settings/sudo/
                cmd.args(["cmd.exe", "/c", "rem"]);
            }
            SudoKind::Gsudo => {
                // `gsudo` doesn't have anything like `sudo -v` to cache credentials,
                // so we just execute a dummy command - the easiest on Windows is
                // `rem` in cmd. `-d` tells it to run the command directly, without
                // going through a shell (which could be powershell) first.
                // See: https://gerardog.github.io/gsudo/docs/usage
                cmd.args(["-d", "cmd.exe", "/c", "rem"]);
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
            SudoKind::Run0 => {
                // `run0` uses polkit for authentication
                // and thus has the same issues as `pkexec`.
                //
                // See: https://www.freedesktop.org/software/systemd/man/devel/run0.html
                cmd.arg("echo");
            }
            SudoKind::Please => {
                // From `man please`
                //   -w, --warm
                //   Warm the access token and exit.
                cmd.arg("-w");
            }
        }
        cmd.status_checked().wrap_err("Failed to elevate permissions")
    }

    /// Execute a command with `sudo`.
    pub fn execute<S: AsRef<OsStr>>(&self, ctx: &ExecutionContext, command: S) -> Result<Executor> {
        self.execute_opts(ctx, command, SudoExecuteOpts::default())
    }

    /// Execute a command with `sudo`, with custom options.
    pub fn execute_opts<S: AsRef<OsStr>>(
        &self,
        ctx: &ExecutionContext,
        command: S,
        opts: SudoExecuteOpts,
    ) -> Result<Executor> {
        let mut cmd = ctx.execute(&self.path);

        if opts.interactive {
            match self.kind {
                SudoKind::Sudo if cfg!(not(target_os = "windows")) => {
                    cmd.arg("-i");
                }
                SudoKind::Gsudo => {
                    // By default, gsudo runs all commands inside a shell, so it's effectively
                    // always "interactive". If interactive is *not* specified, we add `-d`
                    // to run outside of a shell - see below.
                }
                SudoKind::Doas | SudoKind::Sudo | SudoKind::Pkexec | SudoKind::Run0 | SudoKind::Please => {
                    return Err(UnsupportedSudo {
                        sudo_kind: self.kind,
                        option: "interactive",
                    }
                    .into());
                }
            }
        } else if let SudoKind::Gsudo = self.kind {
            // The `-d` (direct) flag disables shell detection, running the command directly
            // rather than through the current shell, making it "non-interactive".
            // Additionally, if the current shell is pwsh >= 7.3.0, then not including this
            // gives errors if the command to run has spaces in it: see
            // https://github.com/gerardog/gsudo/issues/297
            cmd.arg("-d");
        }

        if let Some(preserve_env) = opts.preserve_env {
            if preserve_env.is_empty() {
                match self.kind {
                    SudoKind::Sudo => {
                        cmd.arg("-E");
                    }
                    SudoKind::Gsudo => {
                        cmd.arg("--copyEV");
                    }
                    SudoKind::Doas | SudoKind::Pkexec | SudoKind::Run0 | SudoKind::Please => {
                        return Err(UnsupportedSudo {
                            sudo_kind: self.kind,
                            option: "preserve_env",
                        }
                        .into());
                    }
                }
            } else {
                match self.kind {
                    SudoKind::Sudo if cfg!(not(target_os = "windows")) => {
                        cmd.arg(format!("--preserve_env={}", preserve_env.join(",")));
                    }
                    SudoKind::Run0 => {
                        for env in preserve_env {
                            cmd.arg(format!("--setenv={}", env));
                        }
                    }
                    SudoKind::Please => {
                        cmd.arg("-a");
                        cmd.arg(preserve_env.join(","));
                    }
                    SudoKind::Doas | SudoKind::Sudo | SudoKind::Gsudo | SudoKind::Pkexec => {
                        return Err(UnsupportedSudo {
                            sudo_kind: self.kind,
                            option: "preserve_env list",
                        }
                        .into());
                    }
                }
            }
        }

        if opts.set_home {
            match self.kind {
                SudoKind::Sudo if cfg!(not(target_os = "windows")) => {
                    cmd.arg("-H");
                }
                SudoKind::Doas
                | SudoKind::Sudo
                | SudoKind::Gsudo
                | SudoKind::Pkexec
                | SudoKind::Run0
                | SudoKind::Please => {
                    return Err(UnsupportedSudo {
                        sudo_kind: self.kind,
                        option: "set_home",
                    }
                    .into());
                }
            }
        }

        if let Some(user) = opts.user {
            match self.kind {
                SudoKind::Sudo if cfg!(not(target_os = "windows")) => {
                    cmd.args(["-u", user]);
                }
                SudoKind::Doas | SudoKind::Gsudo | SudoKind::Run0 | SudoKind::Please => {
                    cmd.args(["-u", user]);
                }
                SudoKind::Pkexec => {
                    cmd.args(["--user", user]);
                }
                SudoKind::Sudo => {
                    // Windows sudo is the only one that doesn't have a `-u` flag
                    return Err(UnsupportedSudo {
                        sudo_kind: self.kind,
                        option: "user",
                    }
                    .into());
                }
            }
        }

        cmd.arg(command);

        Ok(cmd)
    }
}

#[derive(Clone, Copy, Debug, Display, Deserialize, AsRefStr)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SudoKind {
    Doas,
    Sudo,
    Gsudo,
    Pkexec,
    Run0,
    Please,
}
