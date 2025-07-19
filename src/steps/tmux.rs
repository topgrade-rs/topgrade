use std::env;
use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::eyre;
use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use etcetera::base_strategy::BaseStrategy;

use crate::command::CommandExt;
use crate::config::TmuxConfig;
use crate::config::TmuxSessionMode;
use crate::terminal::print_separator;
use crate::{
    execution_context::ExecutionContext,
    utils::{which, PathExt},
};
use crate::{HOME_DIR, XDG_DIRS};

use rust_i18n::t;
#[cfg(unix)]
use std::os::unix::process::CommandExt as _;

// update_plugins path is relative to the TPM path
const UPDATE_PLUGINS: &str = "bin/update_plugins";
// Default TPM path relative to the TMux config directory
const TPM_PATH: &str = "plugins/tpm";

pub fn run_tpm(ctx: &ExecutionContext) -> Result<()> {
    let tpm = match env::var("TMUX_PLUGIN_MANAGER_PATH") {
        // Use `$TMUX_PLUGIN_MANAGER_PATH` if set,
        Ok(var) => PathBuf::from(var).join(UPDATE_PLUGINS),
        Err(_) => {
            // otherwise, use the default XDG location `~/.config/tmux`
            #[cfg(unix)]
            let xdg_path = XDG_DIRS.config_dir().join("tmux").join(TPM_PATH).join(UPDATE_PLUGINS);
            #[cfg(windows)]
            let xdg_path = HOME_DIR.join(".config/tmux").join(TPM_PATH).join(UPDATE_PLUGINS);
            if xdg_path.exists() {
                xdg_path
            } else {
                // or fallback on the standard default location `~/.tmux`.
                HOME_DIR.join(".tmux").join(TPM_PATH).join(UPDATE_PLUGINS)
            }
        }
    }
    .require()?;

    print_separator("tmux plugins");

    ctx.execute(tpm).arg("all").status_checked()
}

struct Tmux {
    tmux: PathBuf,
    args: Option<Vec<String>>,
}

impl Tmux {
    fn new(args: Vec<String>) -> Self {
        Self {
            tmux: which("tmux").expect("Could not find tmux"),
            args: if args.is_empty() { None } else { Some(args) },
        }
    }

    fn build(&self) -> Command {
        let mut command = Command::new(&self.tmux);
        if let Some(args) = self.args.as_ref() {
            command.args(args).env_remove("TMUX");
        }
        command
    }

    fn has_session(&self, session_name: &str) -> Result<bool> {
        Ok(self
            .build()
            .args(["has-session", "-t", session_name])
            .output_checked_with(|_| Ok(()))?
            .status
            .success())
    }

    /// Create a new tmux session with the given name, running the given command.
    /// The command is passed to `sh` (see "shell-command arguments are sh(1) commands" in the
    /// `tmux(1)` man page).
    fn new_session(&self, session_name: &str, window_name: &str, command: &str) -> Result<()> {
        let _ = self
            .build()
            // `-d`: initial size comes from the global `default-size` option (instead
            //       of passing `-x` and `-y` arguments.
            //       (What do those even do?)
            // `-s`: session name
            // `-n`: window name (always `topgrade`)
            .args(["new-session", "-d", "-s", session_name, "-n", window_name, command])
            .output_checked()?;
        Ok(())
    }

    /// Like [`new_session`] but it appends a digit to the session name (if necessary) to
    /// avoid duplicate session names.
    ///
    /// The session name is returned.
    fn new_unique_session(&self, session_name: &str, window_name: &str, command: &str) -> Result<String> {
        let mut session = session_name.to_owned();
        for i in 1.. {
            if !self
                .has_session(&session)
                .context("Error determining if a tmux session exists")?
            {
                self.new_session(&session, window_name, command)
                    .context("Error running Topgrade in tmux")?;
                return Ok(session);
            }
            session = format!("{session_name}-{i}");
        }
        unreachable!()
    }

    /// Create a new window in the given tmux session, running the given command.
    fn new_window(&self, session_name: &str, window_name: &str, command: &str) -> Result<()> {
        self.build()
            // `-d`: initial size comes from the global `default-size` option (instead
            //       of passing `-x` and `-y` arguments.
            //       (What do those even do?)
            // `-s`: session name
            // `-n`: window name
            .args([
                "new-window",
                "-a",
                "-t",
                &format!("{session_name}:{window_name}"),
                "-n",
                window_name,
                command,
            ])
            .env_remove("TMUX")
            .status_checked()
    }

    fn window_indices(&self, session_name: &str) -> Result<Vec<usize>> {
        self.build()
            .args(["list-windows", "-F", "#{window_index}", "-t", session_name])
            .output_checked_utf8()?
            .stdout
            .lines()
            .map(str::parse)
            .collect::<Result<Vec<usize>, _>>()
            .context("Failed to compute tmux windows")
    }
}

pub fn run_in_tmux(config: TmuxConfig) -> Result<()> {
    let command = {
        let mut command = vec![
            String::from("env"),
            String::from("TOPGRADE_KEEP_END=1"),
            String::from("TOPGRADE_INSIDE_TMUX=1"),
        ];
        // TODO: Should we use `topgrade` instead of the first argument here, which may be
        // a local path?
        command.extend(env::args());
        shell_words::join(command)
    };

    let tmux = Tmux::new(config.args);

    // Find an unused session and run `topgrade` in it with the current command's arguments.
    let session_name = "topgrade";
    let window_name = "topgrade";
    let session = tmux.new_unique_session(session_name, window_name, &command)?;

    let is_inside_tmux = env::var("TMUX").is_ok();
    let err = match config.session_mode {
        TmuxSessionMode::AttachIfNotInSession => {
            if is_inside_tmux {
                // Only attach to the newly-created session if we're not currently in a tmux session.
                println!("{}", t!("Topgrade launched in a new tmux session"));
                return Ok(());
            } else {
                tmux.build().args(["attach-session", "-t", &session]).exec()
            }
        }

        TmuxSessionMode::AttachAlways => {
            if is_inside_tmux {
                tmux.build().args(["switch-client", "-t", &session]).exec()
            } else {
                tmux.build().args(["attach-session", "-t", &session]).exec()
            }
        }
    };

    Err(eyre!("{err}")).context("Failed to `execvp(3)` tmux")
}

pub fn run_command(ctx: &ExecutionContext, window_name: &str, command: &str) -> Result<()> {
    let tmux = Tmux::new(ctx.config().tmux_config()?.args);

    if let Some(session_name) = ctx.get_tmux_session() {
        let indices = tmux.window_indices(&session_name)?;
        let last_window = indices
            .iter()
            .last()
            .ok_or_else(|| eyre!("tmux session {session_name} has no windows"))?;
        tmux.new_window(&session_name, &format!("{last_window}"), command)?;
    } else {
        let name = tmux.new_unique_session("topgrade", window_name, command)?;
        ctx.set_tmux_session(name);
    }
    Ok(())
}
