use std::env;
use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use color_eyre::eyre::eyre;

use crate::command::CommandExt;
use crate::config::ZellijConfig;
use crate::config::ZellijSessionMode;
use crate::utils::which;

use rust_i18n::t;
#[cfg(unix)]
use std::os::unix::process::CommandExt as _;

struct Zellij {
    zellij: PathBuf,
    args: Option<Vec<String>>,
}

impl Zellij {
    fn new(args: Vec<String>) -> Self {
        Self {
            zellij: which("zellij").expect("Could not find zellij"),
            args: if args.is_empty() { None } else { Some(args) },
        }
    }

    #[allow(clippy::disallowed_methods)]
    fn build(&self) -> Command {
        let mut command = Command::new(&self.zellij);
        // NB: unlike tmux, zellij seems to nest fine without any env-var wrangling.
        if let Some(args) = self.args.as_ref() {
            command.args(args);
        }
        command
    }
    /// Create a new zellij session with the given name, running the given command.
    fn new_session(&self, session_name: &str) -> Result<()> {
        self.build()
            // see https://zellij.dev/documentation/programmatic-control.html#1-create-a-session
            .args(["attach", "--create-background", session_name])
            .output_checked()?;
        // zellij can create a new background session with the layout we want,
        // but only if given a path to a file with the layout.
        // rather than make a temp-file, we spawn zellij with a default layout, then replace it with ours.

        // for that, we'll need a layout string approximately of form:
        // `layout {tab {pane command="env" {args (env args) "topgrade" (topgrade args);};};}`
        // with all args double-quoted.
        // see https://zellij.dev/documentation/creating-a-layout.html for reference.

        // NB: we don't need to TOPGRADE_KEEP_END like in tmux, since zellij keeps the pane on finish
        let mut env_args = "args \"TOPGRADE_INSIDE_ZELLIJ=1\"".to_owned();
        for arg in env::args() {
            // append double-quoted ` "arg"`, escaping double-quotes inside arg itself
            env_args.push_str(&format!(" \"{}\"", arg.replace("\"", "\\\"")));
        }
        let layout_string =
            format!(r#"layout {{ tab name="topgrade" {{ pane name="topgrade" command="env" {{{env_args};}};}};}}"#);
        self.build()
            .env("ZELLIJ_SESSION_NAME", session_name)
            .args(["action", "override-layout", "--layout-string", &layout_string])
            .output_checked()?;
        Ok(())
    }
    /// Like [`new_session`] but it appends a digit to the session name (if necessary) to
    /// avoid duplicate session names.
    ///
    /// The session name is returned.
    fn new_unique_session(&self, session_name: &str) -> Result<String> {
        self.new_session(session_name)
            .context("Error running Topgrade in zellij")?;
        Ok(session_name.to_owned())
        // TODO: new_unique_session
        // let mut session = session_name.to_owned();
        // for i in 1.. {
        //     if !self
        //         .has_session(&session)
        //         .context("Error determining if a tmux session exists")?
        //     {
        //         self.new_session(&session, command)
        //             .context("Error running Topgrade in tmux")?;
        //         return Ok(session);
        //     }
        //     session = format!("{session_name}-{i}");
        // }
        // unreachable!()
    }
}

pub fn run_in_zellij(config: ZellijConfig) -> Result<()> {
    let zellij = Zellij::new(config.args);

    // Find an unused session and run `topgrade` in it with the current command's arguments.
    let session_name = "topgrade";
    let session = zellij.new_unique_session(session_name)?;

    let is_inside_zellij = env::var("ZELLIJ").is_ok();
    let err = match config.session_mode {
        ZellijSessionMode::AttachIfNotInSession => {
            if is_inside_zellij {
                // Only attach to the newly-created session if we're not currently in a zellij session.
                println!(
                    "{}",
                    t!(
                        "Topgrade launched in a new {multiplexer} session",
                        multiplexer = "zellij"
                    )
                );
                return Ok(());
            } else {
                zellij.build().args(["attach", &session]).exec()
            }
        }
        ZellijSessionMode::AttachAlways => {
            if is_inside_zellij {
                zellij.build().args(["action", "switch-session", &session]).exec()
            } else {
                zellij.build().args(["attach", &session]).exec()
            }
        }
    };

    Err(eyre!("{err}")).context("Failed to `execvp(3)` zellij")
}
