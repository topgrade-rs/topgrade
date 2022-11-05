//! Utilities for command execution
use crate::error::{DryRun, TopgradeError};
use crate::utils::{Check, CheckWithCodes};
use anyhow::Result;
use log::{debug, trace};
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Child, Command, ExitStatus};

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

    /// Create an instance of `Executor` that should run `program`.
    pub fn execute<S: AsRef<OsStr>>(self, program: S) -> Executor {
        match self {
            RunType::Dry => Executor::Dry(DryCommand {
                program: program.as_ref().into(),
                ..Default::default()
            }),
            RunType::Wet => Executor::Wet(Command::new(program)),
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

/// An enum providing a similar interface to `std::process::Command`.
/// If the enum is set to `Wet`, execution will be performed with `std::process::Command`.
/// If the enum is set to `Dry`, execution will just print the command with its arguments.
pub enum Executor {
    Wet(Command),
    Dry(DryCommand),
}

impl Executor {
    /// See `std::process::Command::arg`
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Executor {
        match self {
            Executor::Wet(c) => {
                c.arg(arg);
            }
            Executor::Dry(c) => {
                c.args.push(arg.as_ref().into());
            }
        }

        self
    }

    /// See `std::process::Command::args`
    pub fn args<I, S>(&mut self, args: I) -> &mut Executor
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        match self {
            Executor::Wet(c) => {
                c.args(args);
            }
            Executor::Dry(c) => {
                c.args.extend(args.into_iter().map(|arg| arg.as_ref().into()));
            }
        }

        self
    }

    #[allow(dead_code)]
    /// See `std::process::Command::current_dir`
    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Executor {
        match self {
            Executor::Wet(c) => {
                c.current_dir(dir);
            }
            Executor::Dry(c) => c.directory = Some(dir.as_ref().into()),
        }

        self
    }

    #[allow(dead_code)]
    /// See `std::process::Command::remove_env`
    pub fn env_remove<K>(&mut self, key: K) -> &mut Executor
    where
        K: AsRef<OsStr>,
    {
        match self {
            Executor::Wet(c) => {
                c.env_remove(key);
            }
            Executor::Dry(_) => (),
        }

        self
    }

    #[allow(dead_code)]
    /// See `std::process::Command::env`
    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Executor
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        match self {
            Executor::Wet(c) => {
                c.env(key, val);
            }
            Executor::Dry(_) => (),
        }

        self
    }

    /// See `std::process::Command::spawn`
    pub fn spawn(&mut self) -> Result<ExecutorChild> {
        let result = match self {
            Executor::Wet(c) => {
                debug!("Running {:?}", c);
                c.spawn().map(ExecutorChild::Wet)?
            }
            Executor::Dry(c) => {
                c.dry_run();
                ExecutorChild::Dry
            }
        };

        Ok(result)
    }

    /// See `std::process::Command::output`
    pub fn output(&mut self) -> Result<ExecutorOutput> {
        match self {
            Executor::Wet(c) => Ok(ExecutorOutput::Wet(c.output()?)),
            Executor::Dry(c) => {
                c.dry_run();
                Ok(ExecutorOutput::Dry)
            }
        }
    }

    /// A convinence method for `spawn().wait().check()`.
    /// Returns an error if something went wrong during the execution or if the
    /// process exited with failure.
    pub fn check_run(&mut self) -> Result<()> {
        self.spawn()?.wait()?.check()
    }

    /// An extension of `check_run` that allows you to set a sequence of codes
    /// that can indicate success of a script
    #[allow(dead_code)]
    pub fn check_run_with_codes(&mut self, codes: &[i32]) -> Result<()> {
        self.spawn()?.wait()?.check_with_codes(codes)
    }
}

pub enum ExecutorOutput {
    Wet(std::process::Output),
    Dry,
}

/// A struct represending a command. Trying to execute it will just print its arguments.
#[derive(Default)]
pub struct DryCommand {
    program: OsString,
    args: Vec<OsString>,
    directory: Option<OsString>,
}

impl DryCommand {
    fn dry_run(&self) {
        print!(
            "Dry running: {} {}",
            self.program.to_string_lossy(),
            self.args
                .iter()
                .map(|a| String::from(a.to_string_lossy()))
                .collect::<Vec<String>>()
                .join(" ")
        );
        match &self.directory {
            Some(dir) => println!(" in {}", dir.to_string_lossy()),
            None => println!(),
        };
    }
}

/// The Result of spawn. Contains an actual `std::process::Child` if executed by a wet command.
pub enum ExecutorChild {
    Wet(Child),
    Dry,
}

impl ExecutorChild {
    /// See `std::process::Child::wait`
    pub fn wait(&mut self) -> Result<ExecutorExitStatus> {
        let result = match self {
            ExecutorChild::Wet(c) => c.wait().map(ExecutorExitStatus::Wet)?,
            ExecutorChild::Dry => ExecutorExitStatus::Dry,
        };

        Ok(result)
    }
}

/// The Result of wait. Contains an actual `std::process::ExitStatus` if executed by a wet command.
pub enum ExecutorExitStatus {
    Wet(ExitStatus),
    Dry,
}

impl CheckWithCodes for ExecutorExitStatus {
    fn check_with_codes(self, codes: &[i32]) -> Result<()> {
        match self {
            ExecutorExitStatus::Wet(e) => e.check_with_codes(codes),
            ExecutorExitStatus::Dry => Ok(()),
        }
    }
}

/// Extension methods for `std::process::Command`
pub trait CommandExt {
    /// Run the command, wait for it to complete, check the return code and decode the output as UTF-8.
    fn check_output(&mut self) -> Result<String>;
    fn string_output(&mut self) -> Result<String>;
}

impl CommandExt for Command {
    fn check_output(&mut self) -> Result<String> {
        let output = self.output()?;
        trace!("Output of {:?}: {:?}", self, output);
        let status = output.status;
        if !status.success() {
            let stderr = String::from_utf8(output.stderr).unwrap_or_default();
            return Err(TopgradeError::ProcessFailedWithOutput(status, stderr).into());
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    fn string_output(&mut self) -> Result<String> {
        let output = self.output()?;
        trace!("Output of {:?}: {:?}", self, output);
        Ok(String::from_utf8(output.stdout)?)
    }
}

impl CommandExt for Executor {
    fn check_output(&mut self) -> Result<String> {
        let output = match self.output()? {
            ExecutorOutput::Wet(output) => output,
            ExecutorOutput::Dry => return Err(DryRun().into()),
        };
        let status = output.status;
        if !status.success() {
            let stderr = String::from_utf8(output.stderr).unwrap_or_default();
            return Err(TopgradeError::ProcessFailedWithOutput(status, stderr).into());
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    fn string_output(&mut self) -> Result<String> {
        let output = match self.output()? {
            ExecutorOutput::Wet(output) => output,
            ExecutorOutput::Dry => return Err(DryRun().into()),
        };
        Ok(String::from_utf8(output.stdout)?)
    }
}
