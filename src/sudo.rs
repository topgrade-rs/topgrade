use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use rust_i18n::t;
use serde::Deserialize;
use strum::Display;
use thiserror::Error;
#[cfg(target_os = "windows")]
use tracing::{debug, warn};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;

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

#[derive(Error, Debug)]
pub enum SudoCreateError {
    CannotFindBinary,
    #[cfg(target_os = "windows")]
    WinSudoDisabled,
    #[cfg(target_os = "windows")]
    WinSudoNewWindowMode,
}

impl std::fmt::Display for SudoCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SudoCreateError::CannotFindBinary => {
                write!(f, "{}", t!("Cannot find sudo binary"))
            }
            #[cfg(target_os = "windows")]
            SudoCreateError::WinSudoDisabled => {
                write!(f, "{}", t!("Found Windows Sudo, but it is disabled"))
            }
            #[cfg(target_os = "windows")]
            SudoCreateError::WinSudoNewWindowMode => {
                write!(
                    f,
                    "{}",
                    t!("Found Windows Sudo, but it is using 'In a new window' mode")
                )
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum SudoPreserveEnv<'a> {
    /// Preserve all environment variables.
    All,
    /// Preserve only the specified environment variables.
    Some(&'a [&'a str]),
    /// Preserve no environment variables.
    #[default]
    None,
}

/// Generic sudo options, translated into flags to pass to `sudo`.
/// NOTE: Depending on the sudo kind, OS and system config, some options might be specified by
/// default or unsupported.
#[derive(Clone, Debug, Default)]
pub struct SudoExecuteOpts<'a> {
    /// Run the command "interactively", i.e. inside a login shell.
    pub interactive: bool,
    /// Preserve environment variables across the sudo call.
    pub preserve_env: SudoPreserveEnv<'a>,
    /// Set the HOME environment variable to the target user's home directory.
    pub set_home: bool,
    /// Run the command as a user other than the root user.
    pub user: Option<&'a str>,
}

impl<'a> SudoExecuteOpts<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the command "interactively", i.e. inside a login shell.
    #[allow(unused)]
    pub fn interactive(mut self) -> Self {
        self.interactive = true;
        self
    }

    /// Preserve all environment variables across the sudo call.
    #[allow(unused)]
    pub fn preserve_env(mut self) -> Self {
        self.preserve_env = SudoPreserveEnv::All;
        self
    }

    /// Preserve only the specified environment variables across the sudo call.
    #[allow(unused)]
    pub fn preserve_env_list(mut self, vars: &'a [&'a str]) -> Self {
        self.preserve_env = SudoPreserveEnv::Some(vars);
        self
    }

    /// Set the HOME environment variable to the target user's home directory.
    #[allow(unused)]
    pub fn set_home(mut self) -> Self {
        self.set_home = true;
        self
    }

    /// Run the command as a user other than the root user.
    #[allow(unused)]
    pub fn user(mut self, user: &'a str) -> Self {
        self.user = Some(user);
        self
    }
}

#[cfg(not(target_os = "windows"))]
const DETECT_ORDER: [SudoKind; 5] = [
    SudoKind::Doas,
    SudoKind::Sudo,
    SudoKind::Pkexec,
    SudoKind::Run0,
    SudoKind::Please,
];

// NOTE: keep WinSudo last, allows short-circuit error return in Sudo::detect() to work
#[cfg(target_os = "windows")]
const DETECT_ORDER: [SudoKind; 2] = [SudoKind::Gsudo, SudoKind::WinSudo];

impl Sudo {
    /// Get the `sudo` binary for this platform.
    pub fn detect() -> Result<Self, SudoCreateError> {
        use SudoCreateError::*;

        for kind in DETECT_ORDER {
            match Self::new(kind) {
                Ok(sudo) => return Ok(sudo),
                Err(CannotFindBinary) => continue,
                #[cfg(target_os = "windows")]
                Err(e @ (WinSudoDisabled | WinSudoNewWindowMode)) => {
                    // we can return directly here since WinSudo is detected last
                    return Err(e);
                }
            }
        }
        Err(CannotFindBinary)
    }

    /// Create Sudo from SudoKind, if found in the system
    pub fn new(kind: SudoKind) -> Result<Self, SudoCreateError> {
        match kind.which() {
            Some(path) => {
                let sudo = Self { path, kind };

                #[cfg(target_os = "windows")]
                if let SudoKind::WinSudo = kind {
                    // Windows Sudo might be disabled, causing it to error on use.
                    //
                    // It checks two registry keys to determine its mode:
                    // a "policy" (HLKM\SOFTWARE\Policies\Microsoft\Windows\Sudo\Enabled)
                    // and a "setting" (HLKM\SOFTWARE\Microsoft\Windows\CurrentVersion\Sudo\Enabled).
                    //
                    // Both keys are u32's, with these meanings:
                    // 0 - Disabled
                    // 1 - ForceNewWindow
                    // 2 - DisableInput
                    // 3 - Normal
                    //
                    // Setting the sudo option in Settings changes the setting key, the policy key
                    // sets an upper limit on the setting key: mode = min(policy, setting).
                    // The default for the policy key is 3 (all modes allowed), and the default for
                    // the setting key is 0 (disabled).
                    //
                    // See https://github.com/microsoft/sudo/blob/9f50d79704a9d4d468bc59f725993714762981ca/sudo/src/helpers.rs#L442

                    let policy_mode = match windows_registry::LOCAL_MACHINE
                        .open(r"SOFTWARE\Policies\Microsoft\Windows\Sudo")
                        .and_then(|key| key.get_u32("Enabled"))
                    {
                        Ok(v) => v.min(3),
                        // key missing, default to 3/normal
                        Err(e) if e.code() == ERROR_FILE_NOT_FOUND.to_hresult() => 3,
                        Err(e) => {
                            // warn, but treat as 3/normal (using sudo should error)
                            warn!("Error reading registry key: {e}");
                            3
                        }
                    };
                    let setting_mode = match windows_registry::LOCAL_MACHINE
                        .open(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Sudo")
                        .and_then(|key| key.get_u32("Enabled"))
                    {
                        Ok(v) => v.min(3),
                        // key missing, default to 0/disabled
                        Err(e) if e.code() == ERROR_FILE_NOT_FOUND.to_hresult() => 0,
                        Err(e) => {
                            // warn, but treat as 3/normal (using sudo should error)
                            warn!("Error reading registry key: {e}");
                            3
                        }
                    };

                    let sudo_mode = policy_mode.min(setting_mode);
                    debug!("Windows Sudo mode: {sudo_mode}");

                    if sudo_mode == 0 {
                        // sudo is disabled
                        return Err(SudoCreateError::WinSudoDisabled);
                    } else if sudo_mode == 1 {
                        // ForceNewWindow mode
                        return Err(SudoCreateError::WinSudoNewWindowMode);
                    }
                    // Normal mode is best, but DisableInput doesn't seem to cause issues
                }

                Ok(sudo)
            }
            None => Err(SudoCreateError::CannotFindBinary),
        }
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
        // skip if using null sudo
        if let SudoKind::Null = self.kind {
            return Ok(());
        }

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
            SudoKind::Sudo => {
                // From `man sudo` on macOS:
                //   -v, --validate
                //   Update the user's cached credentials, authenticating the user
                //   if necessary.  For the sudoers plugin, this extends the sudo
                //   timeout for another 5 minutes by default, but does not run a
                //   command.  Not all security policies support cached credentials.
                cmd.arg("-v");
            }
            SudoKind::WinSudo => {
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
            SudoKind::Null => unreachable!(),
        }
        cmd.status_checked().wrap_err("Failed to elevate permissions")
    }

    /// Execute a command with `sudo`.
    pub fn execute<S: AsRef<OsStr>>(&self, ctx: &ExecutionContext, command: S) -> Result<Executor> {
        self.execute_opts(ctx, command, SudoExecuteOpts::new())
    }

    /// Execute a command with `sudo`, with custom options.
    pub fn execute_opts<S: AsRef<OsStr>>(
        &self,
        ctx: &ExecutionContext,
        command: S,
        opts: SudoExecuteOpts,
    ) -> Result<Executor> {
        // null sudo is very different, do separately
        if let SudoKind::Null = self.kind {
            if opts.interactive {
                return Err(UnsupportedSudo {
                    sudo_kind: self.kind,
                    option: "interactive",
                }
                .into());
            }
            if opts.user.is_some() {
                return Err(UnsupportedSudo {
                    sudo_kind: self.kind,
                    option: "user",
                }
                .into());
            }

            // NOTE: we ignore preserve_env and set_home, using
            // no sudo effectively preserves these by default

            // run command directly
            return Ok(ctx.execute(command));
        }

        let mut cmd = ctx.execute(&self.path);

        if opts.interactive {
            match self.kind {
                SudoKind::Sudo => {
                    cmd.arg("-i");
                }
                SudoKind::Gsudo => {
                    // By default, gsudo runs all commands inside a shell, so it's effectively
                    // always "interactive". If interactive is *not* specified, we add `-d`
                    // to run outside of a shell - see below.
                }
                SudoKind::Doas | SudoKind::WinSudo | SudoKind::Pkexec | SudoKind::Run0 | SudoKind::Please => {
                    return Err(UnsupportedSudo {
                        sudo_kind: self.kind,
                        option: "interactive",
                    }
                    .into());
                }
                SudoKind::Null => unreachable!(),
            }
        } else if let SudoKind::Gsudo = self.kind {
            // The `-d` (direct) flag disables shell detection, running the command directly
            // rather than through the current shell, making it "non-interactive".
            // Additionally, if the current shell is pwsh >= 7.3.0, then not including this
            // gives errors if the command to run has spaces in it: see
            // https://github.com/gerardog/gsudo/issues/297
            cmd.arg("-d");
        }

        match opts.preserve_env {
            SudoPreserveEnv::All => match self.kind {
                SudoKind::Sudo => {
                    cmd.arg("-E");
                }
                SudoKind::Gsudo => {
                    cmd.arg("--copyEV");
                }
                SudoKind::Doas | SudoKind::WinSudo | SudoKind::Pkexec | SudoKind::Run0 | SudoKind::Please => {
                    return Err(UnsupportedSudo {
                        sudo_kind: self.kind,
                        option: "preserve_env",
                    }
                    .into());
                }
                SudoKind::Null => unreachable!(),
            },
            SudoPreserveEnv::Some(vars) => match self.kind {
                SudoKind::Sudo => {
                    cmd.arg(format!("--preserve-env={}", vars.join(",")));
                }
                SudoKind::Run0 => {
                    for env in vars {
                        cmd.arg(format!("--setenv={}", env));
                    }
                }
                SudoKind::Please => {
                    cmd.arg("-a");
                    cmd.arg(vars.join(","));
                }
                SudoKind::Doas | SudoKind::WinSudo | SudoKind::Gsudo | SudoKind::Pkexec => {
                    return Err(UnsupportedSudo {
                        sudo_kind: self.kind,
                        option: "preserve_env_list",
                    }
                    .into());
                }
                SudoKind::Null => unreachable!(),
            },
            SudoPreserveEnv::None => {}
        }

        if opts.set_home {
            match self.kind {
                SudoKind::Sudo => {
                    cmd.arg("-H");
                }
                SudoKind::Doas
                | SudoKind::WinSudo
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
                SudoKind::Null => unreachable!(),
            }
        }

        if let Some(user) = opts.user {
            match self.kind {
                SudoKind::Sudo => {
                    cmd.args(["-u", user]);
                }
                SudoKind::Doas | SudoKind::Gsudo | SudoKind::Run0 | SudoKind::Please => {
                    cmd.args(["-u", user]);
                }
                SudoKind::Pkexec => {
                    cmd.args(["--user", user]);
                }
                SudoKind::WinSudo => {
                    // Windows sudo is the only one that doesn't have a `-u` flag
                    return Err(UnsupportedSudo {
                        sudo_kind: self.kind,
                        option: "user",
                    }
                    .into());
                }
                SudoKind::Null => unreachable!(),
            }
        }

        cmd.arg(command);

        Ok(cmd)
    }
}

// We need separate `SudoKind` definitions for windows and unix,
// so that we can have serde instantiate `WinSudo` on windows and
// `Sudo` on unix when reading "sudo" from the config file.
// NOTE: when adding a new variant or otherwise changing `SudoKind`,
// make sure to keep both definitions in sync.

#[derive(Clone, Copy, Debug, Display, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[cfg(target_os = "windows")]
pub enum SudoKind {
    Doas,
    #[expect(unused, reason = "Sudo is unix-only")]
    #[serde(skip)]
    Sudo,
    #[serde(rename = "sudo")]
    WinSudo,
    Gsudo,
    Pkexec,
    Run0,
    Please,
    /// A "no-op" sudo, used when topgrade itself is running as Administrator
    Null,
}

#[derive(Clone, Copy, Debug, Display, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[cfg(not(target_os = "windows"))]
pub enum SudoKind {
    Doas,
    Sudo,
    #[expect(unused, reason = "WinSudo is windows-only")]
    #[serde(skip)]
    WinSudo,
    Gsudo,
    Pkexec,
    Run0,
    Please,
    /// A "no-op" sudo, used when topgrade itself is running as root
    Null,
}

impl SudoKind {
    fn binary_name(self) -> &'static str {
        match self {
            SudoKind::Doas => "doas",
            SudoKind::Sudo => "sudo",
            // hardcode the path to ensure we find Windows Sudo
            // rather than gsudo masquerading as sudo
            SudoKind::WinSudo => r"C:\Windows\System32\sudo.exe",
            SudoKind::Gsudo => "gsudo",
            SudoKind::Pkexec => "pkexec",
            SudoKind::Run0 => "run0",
            SudoKind::Please => "please",
            SudoKind::Null => "<null>", // dummy sudo, no actual binary
        }
    }

    fn which(self) -> Option<PathBuf> {
        match self {
            SudoKind::Null => Some(self.binary_name().into()), // no actual binary for null sudo
            _ => which(self.binary_name()),
        }
    }
}
