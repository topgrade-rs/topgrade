#![allow(dead_code)]
use crate::executor::DryCommand;
use crate::sudo::Sudo;
use crate::utils::{get_require_sudo_string, require_option};
use crate::{config::Config, executor::Executor};
use color_eyre::eyre::Result;
use std::env::var;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

/// An enum telling whether Topgrade should perform dry runs or actually perform the steps.
#[derive(Clone, Copy, Debug)]
pub enum RunType {
    /// Executing commands will just print the command with its argument.
    Dry,

    /// Executing commands will perform actual execution.
    Wet,
}

impl RunType {
    /// Create a new instance from a boolean telling whether to dry run.
    pub fn new(dry_run: bool) -> Self {
        if dry_run {
            RunType::Dry
        } else {
            RunType::Wet
        }
    }

    /// Tells whether we're performing a dry run.
    pub fn dry(self) -> bool {
        match self {
            RunType::Dry => true,
            RunType::Wet => false,
        }
    }
}

pub struct ExecutionContext<'a> {
    run_type: RunType,
    sudo: Option<Sudo>,
    config: &'a Config,
    /// Name of a tmux session to execute commands in, if any.
    /// This is used in `./steps/remote/ssh.rs`, where we want to run `topgrade` in a new
    /// tmux window for each remote.
    tmux_session: Mutex<Option<String>>,
    /// True if topgrade is running under ssh.
    under_ssh: bool,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(run_type: RunType, sudo: Option<Sudo>, config: &'a Config) -> Self {
        let under_ssh = var("SSH_CLIENT").is_ok() || var("SSH_TTY").is_ok();
        Self {
            run_type,
            sudo,
            config,
            tmux_session: Mutex::new(None),
            under_ssh,
        }
    }

    /// Create an instance of `Executor` that should run `program`.
    pub fn execute<S: AsRef<OsStr>>(&self, program: S) -> Executor {
        match self.run_type {
            RunType::Dry => Executor::Dry(DryCommand::new(program)),
            RunType::Wet => Executor::Wet(Command::new(program)),
        }
    }

    /// Create an instance of `Executor` that should run `program`,
    /// using sudo to elevate privileges.
    pub fn execute_elevated(&self, command: &Path, interactive: bool) -> Result<Executor> {
        let sudo = require_option(self.sudo.as_ref(), get_require_sudo_string())?;
        Ok(sudo.execute_elevated(self, command, interactive))
    }

    pub fn run_type(&self) -> RunType {
        self.run_type
    }

    pub fn sudo(&self) -> &Option<Sudo> {
        &self.sudo
    }

    pub fn config(&self) -> &Config {
        self.config
    }

    pub fn under_ssh(&self) -> bool {
        self.under_ssh
    }

    pub fn set_tmux_session(&self, session_name: String) {
        self.tmux_session.lock().unwrap().replace(session_name);
    }

    pub fn get_tmux_session(&self) -> Option<String> {
        self.tmux_session.lock().unwrap().clone()
    }
}
