#![allow(dead_code)]
use color_eyre::eyre::Result;
use rust_i18n::t;
use std::env::var;
use std::ffi::OsStr;
use std::process::Command;
use std::sync::{LazyLock, Mutex};

use crate::executor::DryCommand;
use crate::powershell::Powershell;
#[cfg(target_os = "linux")]
use crate::steps::linux::Distribution;
use crate::sudo::Sudo;
use crate::utils::require_option;
use crate::{config::Config, executor::Executor};

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
    #[cfg(target_os = "linux")]
    distribution: &'a Result<Distribution>,
    powershell: LazyLock<Option<Powershell>>,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(
        run_type: RunType,
        sudo: Option<Sudo>,
        config: &'a Config,
        #[cfg(target_os = "linux")] distribution: &'a Result<Distribution>,
    ) -> Self {
        let under_ssh = var("SSH_CLIENT").is_ok() || var("SSH_TTY").is_ok();
        Self {
            run_type,
            sudo,
            config,
            tmux_session: Mutex::new(None),
            under_ssh,
            #[cfg(target_os = "linux")]
            distribution,
            powershell: LazyLock::new(Powershell::new),
        }
    }

    /// Create an instance of `Executor` that should run `program`.
    pub fn execute<S: AsRef<OsStr>>(&self, program: S) -> Executor {
        match self.run_type {
            RunType::Dry => Executor::Dry(DryCommand::new(program)),
            RunType::Wet => Executor::Wet(Command::new(program)),
        }
    }

    pub fn run_type(&self) -> RunType {
        self.run_type
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

    #[cfg(target_os = "linux")]
    pub fn distribution(&self) -> &Result<Distribution> {
        self.distribution
    }

    pub fn powershell(&self) -> &Option<Powershell> {
        &self.powershell
    }

    pub fn require_powershell(&self) -> Result<&Powershell> {
        require_option(self.powershell.as_ref(), t!("Powershell is not installed").to_string())
    }
}
