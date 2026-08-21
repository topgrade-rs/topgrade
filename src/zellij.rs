use std::collections::HashSet;
use std::env;
use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use color_eyre::eyre::eyre;

use crate::command::CommandExt;
use crate::config::ZellijConfig;
use crate::config::ZellijSessionMode;
use crate::execution_context::ExecutionContext;
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
    /// Create a new zellij session with the given name, running `command` with `args` in a tab
    /// (and pane) named `tab_name`.
    fn new_session(&self, session_name: &str, tab_name: &str, command: &str, args: &[&str]) -> Result<()> {
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
        let mut args_kdl = String::new();
        for arg in args {
            // append double-quoted ` "arg"`, escaping double-quotes inside arg itself
            args_kdl.push_str(&format!(" \"{}\"", arg.replace("\"", "\\\"")));
        }
        let layout_string = format!(
            r#"layout {{ tab name="{tab_name}" {{ pane name="{tab_name}" command="{command}" {{ args {args_kdl}; }}; }}; }}"#
        );
        self.build()
            .env("ZELLIJ_SESSION_NAME", session_name)
            .args(["action", "override-layout", "--layout-string", &layout_string])
            .output_checked()?;
        Ok(())
    }
    /// Add a new tab named `tab_name` to the (possibly background) session `session_name`,
    /// running `command` with `args`.
    fn new_tab(&self, session_name: &str, tab_name: &str, command: &str, args: &[&str]) -> Result<()> {
        self.build()
            .env("ZELLIJ_SESSION_NAME", session_name)
            .args(["action", "new-tab", "-n", tab_name, "--", command])
            .args(args)
            .output_checked()?;
        Ok(())
    }

    /// Names of all zellij sessions, including EXITED ones (which still occupy their name).
    fn session_names(&self) -> Result<HashSet<String>> {
        let output = self
            .build()
            .args(["list-sessions", "--short", "--no-formatting"])
            // exits with status 1 when there are no sessions, which is fine
            .output_checked_with_utf8(|_| Ok(()))
            .context("Error listing zellij sessions")?;
        Ok(output.stdout.lines().map(str::to_owned).collect())
    }

    /// Like [`new_session`] but it appends a digit to the session name (if necessary) to
    /// avoid duplicate session names.
    ///
    /// The session name is returned.
    fn new_unique_session(&self, session_name: &str, tab_name: &str, command: &str, args: &[&str]) -> Result<String> {
        let existing = self.session_names().context("Error listing zellij sessions")?;
        let mut session = session_name.to_owned();
        for i in 1.. {
            if !existing.contains(&session) {
                self.new_session(&session, tab_name, command, args)
                    .context("Error running Topgrade in zellij")?;
                return Ok(session);
            }
            session = format!("{session_name}-{i}");
        }
        unreachable!()
    }
}

pub fn run_in_zellij(config: ZellijConfig) -> Result<()> {
    let zellij = Zellij::new(config.args);

    // Find an unused session and run `topgrade` in it with the current command's arguments.
    let session_name = "topgrade";
    // we want zellij to run a "env TOPGRADE_INSIDE_ZELLIJ=1 (current topgrade invocation)".
    // "env" we supply as a command separately, and the invocation we get from env::args().
    // NB: we don't need to TOPGRADE_KEEP_END like in tmux, since zellij keeps the pane on finish
    let mut relaunch_args = vec!["TOPGRADE_INSIDE_ZELLIJ=1".to_owned()];
    relaunch_args.extend(env::args());
    let relaunch_args: Vec<&str> = relaunch_args.iter().map(String::as_str).collect();
    let session = zellij.new_unique_session(session_name, "topgrade", "env", &relaunch_args)?;

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

/// Run `command` with `args` in a new zellij tab named `tab_name`, reusing the session tracked
/// on `ctx` (across `ssh_step` calls for successive remotes) if one exists, or starting one
/// otherwise.
pub fn run_command(ctx: &ExecutionContext, tab_name: &str, command: &str, args: &[&str]) -> Result<()> {
    let zellij = Zellij::new(ctx.config().zellij_config()?.args);

    if let Some(session_name) = ctx.get_zellij_session() {
        zellij.new_tab(&session_name, tab_name, command, args)?;
    } else {
        let name = zellij.new_unique_session("topgrade", tab_name, command, args)?;
        ctx.set_zellij_session(name);
    }
    Ok(())
}
