//! Utilities for command execution
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output};

use color_eyre::eyre::Result;
use tracing::debug;

use crate::command::CommandExt;
use crate::error::DryRun;

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
    /// Get the name of the program being run.
    ///
    /// Will give weird results for non-UTF-8 programs; see `to_string_lossy()`.
    pub fn get_program(&self) -> String {
        match self {
            Executor::Wet(c) => c.get_program().to_string_lossy().into_owned(),
            Executor::Dry(c) => c.program.to_string_lossy().into_owned(),
        }
    }

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
                c.spawn_checked().map(ExecutorChild::Wet)?
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
            Executor::Wet(c) => Ok(ExecutorOutput::Wet(c.output_checked()?)),
            Executor::Dry(c) => {
                c.dry_run();
                Ok(ExecutorOutput::Dry)
            }
        }
    }

    /// An extension of `status_checked` that allows you to set a sequence of codes
    /// that can indicate success of a script
    #[allow(dead_code)]
    pub fn status_checked_with_codes(&mut self, codes: &[i32]) -> Result<()> {
        match self {
            Executor::Wet(c) => c.status_checked_with(|status| {
                if status.success() || status.code().as_ref().map(|c| codes.contains(c)).unwrap_or(false) {
                    Ok(())
                } else {
                    Err(())
                }
            }),
            Executor::Dry(c) => {
                c.dry_run();
                Ok(())
            }
        }
    }
}

pub enum ExecutorOutput {
    Wet(Output),
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
            shell_words::join(
                self.args
                    .iter()
                    .map(|a| String::from(a.to_string_lossy()))
                    .collect::<Vec<String>>()
            )
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

impl CommandExt for Executor {
    type Child = ExecutorChild;

    // TODO: It might be nice to make `output_checked_with` return something that has a
    // variant for wet/dry runs.

    fn output_checked_with(&mut self, succeeded: impl Fn(&Output) -> Result<(), ()>) -> Result<Output> {
        match self {
            Executor::Wet(c) => c.output_checked_with(succeeded),
            Executor::Dry(c) => {
                c.dry_run();
                Err(DryRun().into())
            }
        }
    }

    fn status_checked_with(&mut self, succeeded: impl Fn(ExitStatus) -> Result<(), ()>) -> Result<()> {
        match self {
            Executor::Wet(c) => c.status_checked_with(succeeded),
            Executor::Dry(c) => {
                c.dry_run();
                Ok(())
            }
        }
    }

    fn spawn_checked(&mut self) -> Result<Self::Child> {
        self.spawn()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn test_executor_spawn_wet() {
        let mut executor = RunType::Wet.execute("echo");
        let child = executor.spawn().expect("Failed to spawn child process");
        match child {
            ExecutorChild::Wet(_) => (),
            ExecutorChild::Dry => panic!("Expected a wet executor child"),
        }
    }

    #[test]
    fn test_executor_spawn_dry() {
        let mut executor = RunType::Dry.execute("echo");
        let child = executor.spawn().expect("Failed to spawn child process");
        match child {
            ExecutorChild::Dry => (),
            ExecutorChild::Wet(_) => panic!("Expected a dry executor child"),
        }
    }

    #[test]
    fn test_executor_output_wet() {
        let mut executor = RunType::Wet.execute("echo");
        let output = executor.output().expect("Failed to get output");
        match output {
            ExecutorOutput::Wet(output) => assert!(!output.stdout.is_empty()),
            ExecutorOutput::Dry => panic!("Expected a wet executor output"),
        }
    }

    #[test]
    fn test_executor_output_dry() {
        let mut executor = RunType::Dry.execute("echo");
        let output = executor.output().expect("Failed to get output");
        match output {
            ExecutorOutput::Dry => (),
            ExecutorOutput::Wet(_) => panic!("Expected a dry executor output"),
        }
    }

    #[test]
    fn test_executor_status_checked_with_codes() {
        let mut executor = RunType::Wet.execute("false");
        let result = executor.status_checked_with_codes(&[1]);
        assert!(result.is_ok(), "Expected status code 1 to be treated as success");
    }
}
