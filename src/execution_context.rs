#![allow(dead_code)]
use crate::executor::DryCommand;
use crate::sudo::Sudo;
use crate::utils::require_option;
use crate::{config::Config, executor::Executor};
use color_eyre::eyre::Result;
use rust_i18n::t;
use std::env::var;
use std::ffi::OsStr;
use std::process::Command;
use std::sync::Mutex;

pub struct ExecutionContext<'a> {
    dry_run: bool,
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
    pub fn new(dry_run: bool, sudo: Option<Sudo>, config: &'a Config) -> Self {
        let under_ssh = var("SSH_CLIENT").is_ok() || var("SSH_TTY").is_ok();
        Self {
            dry_run,
            sudo,
            config,
            tmux_session: Mutex::new(None),
            under_ssh,
        }
    }

    /// Create an instance of `Executor` that should run `program`.
    pub fn execute<S: AsRef<OsStr>>(&self, program: S) -> Executor {
        if self.dry_run {
            Executor::Dry(DryCommand::new(program))
        } else {
            Executor::Wet(Command::new(program))
        }
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn sudo(&self) -> &Option<Sudo> {
        &self.sudo
    }

    pub fn require_sudo(&self) -> Result<&Sudo> {
        require_option(
            self.sudo.as_ref(),
            t!("Require sudo or counterpart but not found, skip").to_string(),
        )
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
