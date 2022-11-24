#![allow(dead_code)]
use crate::executor::RunType;
use crate::git::Git;
use crate::utils::require_option;
use crate::{config::Config, executor::Executor};
use color_eyre::eyre::Result;
use directories::BaseDirs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct ExecutionContext<'a> {
    run_type: RunType,
    sudo: &'a Option<PathBuf>,
    git: &'a Git,
    config: &'a Config,
    base_dirs: &'a BaseDirs,
    /// Name of a tmux session to execute commands in, if any.
    /// This is used in `./steps/remote/ssh.rs`, where we want to run `topgrade` in a new
    /// tmux window for each remote.
    tmux_session: Mutex<Option<String>>,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(
        run_type: RunType,
        sudo: &'a Option<PathBuf>,
        git: &'a Git,
        config: &'a Config,
        base_dirs: &'a BaseDirs,
    ) -> Self {
        Self {
            run_type,
            sudo,
            git,
            config,
            base_dirs,
            tmux_session: Mutex::new(None),
        }
    }

    pub fn execute_elevated(&self, command: &Path, interactive: bool) -> Result<Executor> {
        let sudo = require_option(self.sudo.clone(), "Sudo is required for this operation".into())?;
        let mut cmd = self.run_type.execute(&sudo);

        if sudo.ends_with("sudo") {
            cmd.arg("--preserve-env=DIFFPROG");
        }

        if interactive {
            cmd.arg("-i");
        }

        cmd.arg(command);
        Ok(cmd)
    }

    pub fn run_type(&self) -> RunType {
        self.run_type
    }

    pub fn git(&self) -> &Git {
        self.git
    }

    pub fn sudo(&self) -> &Option<PathBuf> {
        self.sudo
    }

    pub fn config(&self) -> &Config {
        self.config
    }

    pub fn base_dirs(&self) -> &BaseDirs {
        self.base_dirs
    }

    pub fn set_tmux_session(&self, session_name: String) {
        self.tmux_session.lock().unwrap().replace(session_name);
    }

    pub fn get_tmux_session(&self) -> Option<String> {
        self.tmux_session.lock().unwrap().clone()
    }
}
