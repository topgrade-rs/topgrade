#![allow(dead_code)]
use crate::executor::RunType;
use crate::git::Git;
use crate::sudo::Sudo;
use crate::utils::{require_option, REQUIRE_SUDO};
use crate::{config::Config, executor::Executor};
use color_eyre::eyre::Result;
use std::env::var;
use std::path::Path;
use std::sync::Mutex;

pub struct ExecutionContext<'a> {
    run_type: RunType,
    sudo: Option<Sudo>,
    git: &'a Git,
    config: &'a Config,
    /// Name of a tmux session to execute commands in, if any.
    /// This is used in `./steps/remote/ssh.rs`, where we want to run `topgrade` in a new
    /// tmux window for each remote.
    tmux_session: Mutex<Option<String>>,
    /// True if topgrade is running under ssh.
    under_ssh: bool,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(run_type: RunType, sudo: Option<Sudo>, git: &'a Git, config: &'a Config) -> Self {
        let under_ssh = var("SSH_CLIENT").is_ok() || var("SSH_TTY").is_ok();
        Self {
            run_type,
            sudo,
            git,
            config,
            tmux_session: Mutex::new(None),
            under_ssh,
        }
    }

    pub fn execute_elevated(&self, command: &Path, interactive: bool) -> Result<Executor> {
        let sudo = require_option(self.sudo.as_ref(), REQUIRE_SUDO.to_string())?;
        Ok(sudo.execute_elevated(self, command, interactive))
    }

    pub fn run_type(&self) -> RunType {
        self.run_type
    }

    pub fn git(&self) -> &Git {
        self.git
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
