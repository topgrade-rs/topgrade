//! Utilities for command execution
use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::iter;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};

use color_eyre::eyre::Result;
use rust_i18n::t;
use tracing::{Level, debug, enabled};

use crate::command::CommandExt;
use crate::error::DryRun;

/// An enum providing a similar interface to `std::process::Command`.
/// If the enum is set to `Wet`, execution will be performed with `std::process::Command`.
/// If the enum is set to `Dry`, execution will just print the command with its arguments.
pub enum Executor {
    Wet(Command),
    Damp(Command),
    Dry(DryCommand),
}

impl Executor {
    /// Convert this executor to always run, even in dry-run mode.
    ///
    /// Use this for read-only commands that detect environment, check versions, or query
    /// configuration. These commands need to run even during dry-run to make correct decisions.
    ///
    /// This converts `Dry` to `Wet` (which executes), while leaving `Wet` and
    /// `Damp` unchanged.
    pub fn always(self) -> Self {
        match self {
            Executor::Dry(c) => Executor::Wet(c.into_command()),
            other => other,
        }
    }

    /// Get the name of the program being run.
    ///
    /// Will give weird results for non-UTF-8 programs; see `to_string_lossy()`.
    pub fn get_program(&self) -> String {
        match self {
            Executor::Wet(c) | Executor::Damp(c) => c.get_program().to_string_lossy().into_owned(),
            Executor::Dry(c) => c.program.to_string_lossy().into_owned(),
        }
    }

    /// See `std::process::Command::arg`
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Executor {
        match self {
            Executor::Wet(c) | Executor::Damp(c) => {
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
            Executor::Wet(c) | Executor::Damp(c) => {
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
            Executor::Wet(c) | Executor::Damp(c) => {
                c.current_dir(dir);
            }
            Executor::Dry(c) => c.directory = Some(dir.as_ref().into()),
        }

        self
    }

    #[allow(dead_code)]
    /// See `std::process::Command::stdin`
    pub fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Executor {
        let stdio = cfg.into();
        match self {
            Executor::Wet(c) | Executor::Damp(c) => {
                c.stdin(stdio);
            }
            Executor::Dry(c) => {
                c.stdin = Some(stdio);
            }
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
            Executor::Wet(c) | Executor::Damp(c) => {
                c.env_remove(key);
            }
            Executor::Dry(c) => {
                c.env_removals.push(key.as_ref().to_os_string());
            }
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
            Executor::Wet(c) | Executor::Damp(c) => {
                c.env(key, val);
            }
            Executor::Dry(c) => {
                c.envs.push((key.as_ref().to_os_string(), val.as_ref().to_os_string()));
            }
        }

        self
    }

    /// See `std::process::Command::spawn`
    pub fn spawn(&mut self) -> Result<ExecutorChild> {
        self.log_command();
        let result = match self {
            Executor::Wet(c) | Executor::Damp(c) => {
                debug!("Running {:?}", c);
                // We should use `spawn()` here rather than `spawn_checked()` since
                // their semantics and behaviors are different.
                #[allow(clippy::disallowed_methods)]
                c.spawn().map(ExecutorChild::Wet)?
            }
            Executor::Dry(_) => ExecutorChild::Dry,
        };

        Ok(result)
    }

    /// See `std::process::Command::output`
    pub fn output(&mut self) -> Result<ExecutorOutput> {
        self.log_command();
        match self {
            Executor::Wet(c) | Executor::Damp(c) => {
                // We should use `output()` here rather than `output_checked()` since
                // their semantics and behaviors are different.
                #[allow(clippy::disallowed_methods)]
                Ok(ExecutorOutput::Wet(c.output()?))
            }
            Executor::Dry(_) => Ok(ExecutorOutput::Dry),
        }
    }

    /// An extension of `status_checked` that allows you to set a sequence of codes
    /// that can indicate success of a script
    #[allow(dead_code)]
    pub fn status_checked_with_codes(&mut self, codes: &[i32]) -> Result<()> {
        self.log_command();
        match self {
            Executor::Wet(c) | Executor::Damp(c) => c.status_checked_with(|status| {
                if status.success() || status.code().as_ref().is_some_and(|c| codes.contains(c)) {
                    Ok(())
                } else {
                    Err(())
                }
            }),
            Executor::Dry(_) => Ok(()),
        }
    }

    fn log_command(&self) {
        match self {
            Executor::Wet(_) => (),
            Executor::Damp(c) => {
                log_command(
                    "Executing: {program_name} {arguments}",
                    c.get_program(),
                    c.get_args(),
                    c.get_envs(),
                    c.get_current_dir(),
                );
            }
            Executor::Dry(c) => log_command(
                "Dry running: {program_name} {arguments}",
                &c.program,
                &c.args,
                iter::empty(),
                c.directory.as_ref(),
            ),
        }
    }
}

pub enum ExecutorOutput {
    Wet(Output),
    Dry,
}

/// A struct representing a command. Trying to execute it will just print its arguments.
pub struct DryCommand {
    program: OsString,
    args: Vec<OsString>,
    directory: Option<OsString>,
    envs: Vec<(OsString, OsString)>,
    env_removals: Vec<OsString>,
    stdin: Option<Stdio>,
}

impl DryCommand {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self {
            program: program.as_ref().to_os_string(),
            args: Vec::new(),
            directory: None,
            envs: Vec::new(),
            env_removals: Vec::new(),
            stdin: None,
        }
    }

    /// Convert this dry command into a real Command that will execute.
    #[allow(clippy::disallowed_methods)]
    fn into_command(self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        if let Some(dir) = &self.directory {
            cmd.current_dir(dir);
        }
        for (key, val) in &self.envs {
            cmd.env(key, val);
        }
        for key in &self.env_removals {
            cmd.env_remove(key);
        }
        if let Some(stdin) = self.stdin {
            cmd.stdin(stdin);
        }
        cmd
    }
}

/// The Result of spawn. Contains an actual `std::process::Child` if executed by a wet command.
pub enum ExecutorChild {
    // Both RunType::Wet and RunType::Damp use this variant
    #[allow(unused)] // this type has not been used
    Wet(Child),
    Dry,
}

impl CommandExt for Executor {
    type Child = ExecutorChild;

    // TODO: It might be nice to make `output_checked_with` return something that has a
    // variant for wet/dry runs.

    fn output_checked_with(&mut self, succeeded: impl Fn(&Output) -> Result<(), ()>) -> Result<Output> {
        self.log_command();
        match self {
            Executor::Wet(c) | Executor::Damp(c) => c.output_checked_with(succeeded),
            Executor::Dry(_) => Err(DryRun().into()),
        }
    }

    fn status_checked_with(&mut self, succeeded: impl Fn(ExitStatus) -> Result<(), ()>) -> Result<()> {
        self.log_command();
        match self {
            Executor::Wet(c) | Executor::Damp(c) => c.status_checked_with(succeeded),
            Executor::Dry(_) => Ok(()),
        }
    }

    fn spawn_checked(&mut self) -> Result<Self::Child> {
        self.spawn()
    }
}

fn log_command<
    'a,
    I: ExactSizeIterator<Item = (&'a (impl Debug + 'a + ?Sized), Option<&'a (impl Debug + 'a + ?Sized)>)>,
>(
    prefix: &str,
    exec: &OsStr,
    args: impl IntoIterator<Item = &'a (impl AsRef<OsStr> + ?Sized + 'a)>,
    env: impl IntoIterator<Item = (&'a OsStr, Option<&'a OsStr>), IntoIter = I>,
    dir: Option<&'a (impl AsRef<Path> + ?Sized)>,
) {
    println!(
        "{}",
        t!(
            prefix,
            program_name = exec.to_string_lossy(),
            arguments = shell_words::join(args.into_iter().map(|s| s.as_ref().to_string_lossy()))
        )
    );

    let env_iter = env.into_iter();
    if env_iter.len() != 0 && enabled!(Level::DEBUG) {
        println!(
            "  {}",
            t!(
                "with env: {env}",
                env = env_iter
                    .filter(|(_, val)| val.is_some())
                    .map(|(key, val)| format!("{:?}={:?}", key, val.unwrap()))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        )
    }

    if let Some(d) = dir {
        println!("  {}", t!("in {directory}", directory = d.as_ref().display()));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Test that a Dry executor converted with .always() actually executes the command.
    #[test]
    fn test_dry_always_executes() {
        let (program, args) = if cfg!(windows) {
            ("cmd", vec!["/C", "echo", "hello"])
        } else {
            ("echo", vec!["hello"])
        };

        let mut dry = DryCommand::new(program);
        for arg in args {
            dry.args.push(arg.into());
        }

        let executor = Executor::Dry(dry);
        let mut executor = executor.always();
        assert!(matches!(executor, Executor::Wet(_)));

        let output = executor.output().unwrap();
        match output {
            ExecutorOutput::Wet(o) => {
                assert!(o.status.success());
                assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "hello");
            }
            ExecutorOutput::Dry => panic!("Expected Wet output after .always()"),
        }
    }

    #[test]
    fn test_dry_stdin_persistence() {
        use std::io::{Seek, Write};

        let (program, args): (&str, Vec<&str>) = if cfg!(windows) {
            ("powershell", vec!["-NoProfile", "-Command", "$input | Write-Output"])
        } else {
            ("cat", vec![])
        };

        let mut dry = DryCommand::new(program);
        for arg in args {
            dry.args.push(arg.into());
        }

        let mut tmpfile = tempfile::tempfile().expect("Failed to create temp file");
        write!(tmpfile, "stdin_test_value").expect("Failed to write to temp file");
        tmpfile.seek(std::io::SeekFrom::Start(0)).expect("Failed to seek");

        let mut executor = Executor::Dry(dry);
        executor.stdin(Stdio::from(tmpfile));
        let mut executor = executor.always();

        let output = executor.output().expect("Failed to execute");
        match output {
            ExecutorOutput::Wet(o) => {
                assert!(o.status.success());
                let stdout = String::from_utf8_lossy(&o.stdout);
                assert!(
                    stdout.contains("stdin_test_value"),
                    "Stdin not persisted through always(): got '{}'",
                    stdout.trim()
                );
            }
            ExecutorOutput::Dry => panic!("Expected Wet output after .always()"),
        }
    }

    /// Test that env vars set on a Dry executor are preserved when converted with .always().
    #[test]
    fn test_dry_env_persistence() {
        let (program, args) = if cfg!(windows) {
            ("cmd", vec!["/C", "echo %TEST_VAR%"])
        } else {
            ("sh", vec!["-c", "echo $TEST_VAR"])
        };

        let mut dry = DryCommand::new(program);
        for arg in args {
            dry.args.push(arg.into());
        }
        dry.envs.push(("TEST_VAR".into(), "preserved_value".into()));

        let executor = Executor::Dry(dry);
        let mut executor = executor.always();

        let output = executor.output().unwrap();
        match output {
            ExecutorOutput::Wet(o) => {
                assert!(o.status.success());
                assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "preserved_value");
            }
            ExecutorOutput::Dry => panic!("Expected Wet output after .always()"),
        }
    }
}
