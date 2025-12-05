use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

#[cfg(windows)]
use color_eyre::eyre;
#[cfg(windows)]
use color_eyre::eyre::eyre;
use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use rust_i18n::t;
use serde::Deserialize;
use strum::Display;
use thiserror::Error;
#[cfg(windows)]
use tracing::{debug, warn};
#[cfg(windows)]
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
    path: Option<PathBuf>,
    /// The type of program being used as `sudo`.
    kind: SudoKind,
}

#[derive(Error, Debug)]
pub enum SudoCreateError {
    CannotFindBinary,
    #[cfg(windows)]
    WinSudoDisabled,
    #[cfg(windows)]
    WinSudoNewWindowMode,
}

impl std::fmt::Display for SudoCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SudoCreateError::CannotFindBinary => {
                write!(f, "{}", t!("Cannot find sudo binary"))
            }
            #[cfg(windows)]
            SudoCreateError::WinSudoDisabled => {
                write!(f, "{}", t!("Found Windows Sudo, but it is disabled"))
            }
            #[cfg(windows)]
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
    Some(Vec<&'a str>),
    /// Preserve no environment variables.
    #[default]
    None,
}

/// Generic sudo options, translated into flags to pass to `sudo`.
/// NOTE: Depending on the sudo kind, OS and system config, some options might be specified by
/// default or unsupported.
#[derive(Clone, Debug, Default)]
pub struct SudoExecuteOpts<'a> {
    /// Run the command inside a login shell.
    pub login_shell: bool,
    /// Preserve environment variables across the sudo call.
    pub preserve_env: SudoPreserveEnv<'a>,
    /// Set the HOME environment variable to the target user's home directory.
    pub set_home: bool,
    /// Run the command as a user other than the root user.
    pub user: Option<&'a str>,
}

impl<'a> SudoExecuteOpts<'a> {
    pub fn new(ctx: &'a ExecutionContext) -> Self {
        // The `--env` arguments are set globally in `main.rs`, but sudo by default
        // does not pass these environment variables through unless explicitly told to.
        // So we add them here to the preserve_env list.
        Self::default().extend_preserve_env_list(
            ctx.config()
                .env_variables()
                .map(|(key, _value)| key),
        )
    }

    /// Run the command inside a login shell.
    #[allow(unused)]
    pub fn login_shell(mut self) -> Self {
        self.login_shell = true;
        self
    }

    /// Preserve all environment variables across the sudo call.
    #[allow(unused)]
    pub fn preserve_env(mut self) -> Self {
        self.preserve_env = SudoPreserveEnv::All;
        self
    }

    /// Preserve the specified environment variables across the sudo call.
    ///
    /// Can be called multiple times to add more variables.
    #[allow(unused)]
    pub fn extend_preserve_env_list<AnyStrIter, StrOrStrRef>(mut self, vars: AnyStrIter) -> Self
    where
        AnyStrIter: IntoIterator<Item = &'a StrOrStrRef>, // Zero-copy reference type, allowing &[&str], &Vec<String>, Vec<&str>, Iter<&str>
        StrOrStrRef: AsRef<str> + ?Sized + 'a,            // coerced to str, &str, &String
    {
        let vars = vars.into_iter().map(|s| s.as_ref());
        match self.preserve_env {
            SudoPreserveEnv::All => {}
            SudoPreserveEnv::Some(ref mut env_list) => env_list.extend(vars),
            SudoPreserveEnv::None => self.preserve_env = SudoPreserveEnv::Some(vars.collect()),
        }
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

#[cfg(not(windows))]
const DETECT_ORDER: [SudoKind; 5] = [
    SudoKind::Doas,
    SudoKind::Sudo,
    SudoKind::Pkexec,
    SudoKind::Run0,
    SudoKind::Please,
];

// NOTE: keep WinSudo last, allows short-circuit error return in Sudo::detect() to work
#[cfg(windows)]
const DETECT_ORDER: [SudoKind; 2] = [SudoKind::Gsudo, SudoKind::WinSudo];

impl Sudo {
    /// Get the `sudo` binary for this platform.
    pub fn detect() -> Result<Self, SudoCreateError> {
        use SudoCreateError::*;

        for kind in DETECT_ORDER {
            match Self::new(kind) {
                Ok(sudo) => return Ok(sudo),
                Err(CannotFindBinary) => continue,
                #[cfg(windows)]
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
        // no actual binary for null sudo
        if let SudoKind::Null = kind {
            return Ok(Self { path: None, kind });
        }

        match kind.which() {
            Some(path) => {
                let sudo = Self { path: Some(path), kind };

                #[cfg(windows)]
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

                    #[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
                    enum SudoMode {
                        Disabled = 0,
                        ForceNewWindow = 1,
                        DisableInput = 2,
                        Normal = 3,
                    }

                    impl TryFrom<u32> for SudoMode {
                        type Error = eyre::Error;

                        fn try_from(value: u32) -> Result<Self> {
                            match value {
                                0 => Ok(SudoMode::Disabled),
                                1 => Ok(SudoMode::ForceNewWindow),
                                2 => Ok(SudoMode::DisableInput),
                                3 => Ok(SudoMode::Normal),
                                _ => Err(eyre!("invalid integer SudoMode: {value}")),
                            }
                        }
                    }

                    fn get_mode(key: &str, on_missing: SudoMode) -> SudoMode {
                        match windows_registry::LOCAL_MACHINE
                            .open(key)
                            .and_then(|k| k.get_u32("Enabled"))
                        {
                            Ok(v) => v.min(3).try_into().unwrap(),
                            Err(e) if e.code() == ERROR_FILE_NOT_FOUND.to_hresult() => on_missing,
                            Err(e) => {
                                // warn, but treat as normal (using sudo should error)
                                warn!(r"Error reading registry key HKLM\{key}\Enabled: {e}");
                                SudoMode::Normal
                            }
                        }
                    }

                    // default to normal if key missing
                    let policy_mode = get_mode(r"SOFTWARE\Policies\Microsoft\Windows\Sudo", SudoMode::Normal);
                    debug!("Windows Sudo policy mode: {policy_mode:?}");
                    // default to disabled if key missing
                    let setting_mode = get_mode(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Sudo", SudoMode::Disabled);
                    debug!("Windows Sudo setting mode: {setting_mode:?}");

                    let sudo_mode = policy_mode.min(setting_mode);
                    debug!("Windows Sudo mode: {sudo_mode:?}");

                    if sudo_mode == SudoMode::Disabled {
                        return Err(SudoCreateError::WinSudoDisabled);
                    } else if sudo_mode == SudoMode::ForceNewWindow {
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
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
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

        // self.path is only None for null sudo, which we've handled above
        let mut cmd = ctx.execute(self.path.as_deref().unwrap());
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
        self.execute_opts(ctx, command, SudoExecuteOpts::new(ctx))
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
            if opts.login_shell {
                // TODO: emulate running in a login shell with su/runuser
                return Err(UnsupportedSudo {
                    sudo_kind: self.kind,
                    option: "login_shell",
                }
                .into());
            }
            if opts.user.is_some() {
                // TODO: emulate running as a different user with su/runuser
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

        // self.path is only None for null sudo, which we've handled above
        let mut cmd = ctx.execute(self.path.as_ref().unwrap());

        if opts.login_shell {
            match self.kind {
                SudoKind::Sudo => {
                    cmd.arg("-i");
                }
                SudoKind::Gsudo => {
                    // By default, gsudo runs all commands inside a shell. If login_shell
                    // is *not* specified, we add `-d` to run outside of a shell - see below.
                }
                SudoKind::Doas | SudoKind::WinSudo | SudoKind::Pkexec | SudoKind::Run0 | SudoKind::Please => {
                    return Err(UnsupportedSudo {
                        sudo_kind: self.kind,
                        option: "login_shell",
                    }
                    .into());
                }
                SudoKind::Null => unreachable!(),
            }
        } else if let SudoKind::Gsudo = self.kind {
            // The `-d` (direct) flag disables shell detection, running the command directly
            // rather than through the current shell.
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

// On unix we use `SudoKind::Sudo`, and on windows `SudoKind::WinSudo`.
// We always define both though, so that we don't have to put
// #[cfg(...)] everywhere.

#[derive(Clone, Copy, Debug, Display, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SudoKind {
    // On unix, "sudo" in the config file means Sudo
    #[cfg(not(windows))]
    Sudo,
    // and WinSudo is skipped, making it unused.
    #[cfg(not(windows))]
    #[expect(unused, reason = "WinSudo is windows-only")]
    #[serde(skip)]
    WinSudo,

    // On unix, Sudo is skipped and unused
    #[cfg(windows)]
    #[expect(unused, reason = "Sudo is unix-only")]
    #[serde(skip)]
    Sudo,
    // and "sudo" in the config file means WinSudo.
    #[cfg(windows)]
    #[serde(rename = "sudo")]
    WinSudo,

    Doas,
    Gsudo,
    Pkexec,
    Run0,
    Please,
    /// A "no-op" sudo, used when topgrade itself is running as root
    Null,
}

impl SudoKind {
    /// Get the name of the "sudo" binary.
    ///
    /// For `SudoKind::WinSudo`, returns the full hardcoded path
    /// instead to ensure we find Windows Sudo rather than gsudo
    /// masquerading as sudo.
    ///
    /// Only returns `None` for `SudoKind::Null`.
    fn binary_name(self) -> Option<&'static str> {
        match self {
            SudoKind::Doas => Some("doas"),
            SudoKind::Sudo => Some("sudo"),
            SudoKind::WinSudo => Some(r"C:\Windows\System32\sudo.exe"),
            SudoKind::Gsudo => Some("gsudo"),
            SudoKind::Pkexec => Some("pkexec"),
            SudoKind::Run0 => Some("run0"),
            SudoKind::Please => Some("please"),
            SudoKind::Null => None,
        }
    }

    /// Find the full path to the "sudo" binary, if it exists on the system.
    fn which(self) -> Option<PathBuf> {
        match self.binary_name() {
            Some(name) => which(name),
            None => None,
        }
    }
}
