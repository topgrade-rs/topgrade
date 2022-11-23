//! Utilities for running commands and providing user-friendly error messages.

use std::fmt::Display;
use std::process::Child;
use std::process::{Command, ExitStatus, Output};

use color_eyre::eyre;
use color_eyre::eyre::eyre;
use color_eyre::eyre::Context;

use crate::error::TopgradeError;

/// Like [`Output`], but UTF-8 decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Utf8Output {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl TryFrom<Output> for Utf8Output {
    type Error = eyre::Error;

    fn try_from(Output { status, stdout, stderr }: Output) -> Result<Self, Self::Error> {
        let stdout = String::from_utf8(stdout).map_err(|err| {
            eyre!(
                "Stdout contained invalid UTF-8: {}",
                String::from_utf8_lossy(err.as_bytes())
            )
        })?;
        let stderr = String::from_utf8(stderr).map_err(|err| {
            eyre!(
                "Stderr contained invalid UTF-8: {}",
                String::from_utf8_lossy(err.as_bytes())
            )
        })?;

        Ok(Utf8Output { status, stdout, stderr })
    }
}

impl TryFrom<&Output> for Utf8Output {
    type Error = eyre::Error;

    fn try_from(Output { status, stdout, stderr }: &Output) -> Result<Self, Self::Error> {
        let stdout = String::from_utf8(stdout.to_vec()).map_err(|err| {
            eyre!(
                "Stdout contained invalid UTF-8: {}",
                String::from_utf8_lossy(err.as_bytes())
            )
        })?;
        let stderr = String::from_utf8(stderr.to_vec()).map_err(|err| {
            eyre!(
                "Stderr contained invalid UTF-8: {}",
                String::from_utf8_lossy(err.as_bytes())
            )
        })?;
        let status = *status;

        Ok(Utf8Output { status, stdout, stderr })
    }
}

impl Display for Utf8Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stdout)
    }
}

/// Extension trait for [`Command`], adding helpers to gather output while checking the exit
/// status.
///
/// These also give us significantly better error messages, which include:
///
/// 1. The command and arguments that were executed, escaped with familiar `sh` syntax.
/// 2. The exit status of the command or the signal that killed it.
/// 3. If we were capturing the output of the command, rather than forwarding it to the user's
///    stdout/stderr, the error message includes the command's stdout and stderr output.
///
/// Additionally, executing commands with these methods will log the command at debug-level,
/// useful when gathering error reports.
pub trait CommandExt {
    type Child;

    /// Like [`Command::output`], but checks the exit status and provides nice error messages.
    ///
    /// Returns an `Err` if the command failed to execute or returned a non-zero exit code.
    #[track_caller]
    fn output_checked(&mut self) -> eyre::Result<Output> {
        self.output_checked_with(|output: &Output| if output.status.success() { Ok(()) } else { Err(()) })
    }

    /// Like [`output_checked`], but also decodes Stdout and Stderr as UTF-8.
    ///
    /// Returns an `Err` if the command failed to execute, returned a non-zero exit code, or if the
    /// output contains invalid UTF-8.
    #[track_caller]
    fn output_checked_utf8(&mut self) -> eyre::Result<Utf8Output> {
        let output = self.output_checked()?;
        output.try_into()
    }

    /// Like [`output_checked`] but a closure determines if the command failed instead of
    /// [`ExitStatus::success`].
    ///
    /// Returns an `Err` if the command failed to execute or if `succeeded` returns an `Err`.
    /// (This lets the caller substitute their own notion of "success" instead of assuming
    /// non-zero exit codes indicate success.)
    #[track_caller]
    fn output_checked_with(&mut self, succeeded: impl Fn(&Output) -> Result<(), ()>) -> eyre::Result<Output>;

    /// Like [`output_checked_with`], but also decodes Stdout and Stderr as UTF-8.
    ///
    /// Returns an `Err` if the command failed to execute, if `succeeded` returns an `Err`, or if
    /// the output contains invalid UTF-8.
    #[track_caller]
    fn output_checked_with_utf8(
        &mut self,
        succeeded: impl Fn(&Utf8Output) -> Result<(), ()>,
    ) -> eyre::Result<Utf8Output> {
        // This decodes the Stdout and Stderr as UTF-8 twice...
        let output =
            self.output_checked_with(|output| output.try_into().map_err(|_| ()).and_then(|o| succeeded(&o)))?;
        output.try_into()
    }

    /// Like [`Command::status`], but gives a nice error message if the status is unsuccessful
    /// rather than returning the [`ExitStatus`].
    ///
    /// Returns `Ok` if the command executes successfully, returns `Err` if the command fails to
    /// execute or returns a non-zero exit code.
    #[track_caller]
    fn status_checked(&mut self) -> eyre::Result<()> {
        self.status_checked_with(|status| if status.success() { Ok(()) } else { Err(()) })
    }

    /// Like [`status_checked`], but gives a nice error message if the status is unsuccessful
    /// rather than returning the [`ExitStatus`].
    ///
    /// Returns `Ok` if the command executes successfully, returns `Err` if the command fails to
    /// execute or if `succeeded` returns an `Err`.
    /// (This lets the caller substitute their own notion of "success" instead of assuming
    /// non-zero exit codes indicate success.)
    #[track_caller]
    fn status_checked_with(&mut self, succeeded: impl Fn(ExitStatus) -> Result<(), ()>) -> eyre::Result<()>;

    /// Like [`Command::spawn`], but gives a nice error message if the command fails to
    /// execute.
    #[track_caller]
    fn spawn_checked(&mut self) -> eyre::Result<Self::Child>;
}

impl CommandExt for Command {
    type Child = Child;

    fn output_checked_with(&mut self, succeeded: impl Fn(&Output) -> Result<(), ()>) -> eyre::Result<Output> {
        let command = log(self);

        // This is where we implement `output_checked`, which is what we prefer to use instead of
        // `output`, so we allow `Command::output` here.
        #[allow(clippy::disallowed_methods)]
        let output = self
            .output()
            .with_context(|| format!("Failed to execute `{command}`"))?;

        if succeeded(&output).is_ok() {
            Ok(output)
        } else {
            let mut message = format!("Command failed: `{command}`");
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            let stdout_trimmed = stdout.trim();
            if !stdout_trimmed.is_empty() {
                message.push_str(&format!("\n\nStdout:\n{stdout_trimmed}"));
            }
            let stderr_trimmed = stderr.trim();
            if !stderr_trimmed.is_empty() {
                message.push_str(&format!("\n\nStderr:\n{stderr_trimmed}"));
            }

            let (program, _) = get_program_and_args(self);
            let err = TopgradeError::ProcessFailedWithOutput(program, output.status, stderr.into_owned());

            let ret = Err(err).with_context(|| message);
            tracing::debug!("Command failed: {ret:?}");
            ret
        }
    }

    fn status_checked_with(&mut self, succeeded: impl Fn(ExitStatus) -> Result<(), ()>) -> eyre::Result<()> {
        let command = log(self);
        let message = format!("Failed to execute `{command}`");

        // This is where we implement `status_checked`, which is what we prefer to use instead of
        // `status`, so we allow `Command::status` here.
        #[allow(clippy::disallowed_methods)]
        let status = self.status().with_context(|| message.clone())?;

        if succeeded(status).is_ok() {
            Ok(())
        } else {
            let (program, _) = get_program_and_args(self);
            let err = TopgradeError::ProcessFailed(program, status);
            let ret = Err(err).with_context(|| format!("Command failed: `{command}`"));
            tracing::debug!("Command failed: {ret:?}");
            ret
        }
    }

    fn spawn_checked(&mut self) -> eyre::Result<Self::Child> {
        let command = log(self);
        let message = format!("Failed to execute `{command}`");

        // This is where we implement `spawn_checked`, which is what we prefer to use instead of
        // `spawn`, so we allow `Command::spawn` here.
        #[allow(clippy::disallowed_methods)]
        {
            self.spawn().with_context(|| message.clone())
        }
    }
}

fn get_program_and_args(cmd: &Command) -> (String, String) {
    // We're not doing anything weird with commands that are invalid UTF-8 so this is fine.
    let program = cmd.get_program().to_string_lossy().into_owned();
    let args = shell_words::join(cmd.get_args().map(|arg| arg.to_string_lossy()));
    (program, args)
}

fn format_program_and_args(cmd: &Command) -> String {
    let (program, args) = get_program_and_args(cmd);
    if args.is_empty() {
        program
    } else {
        format!("{program} {args}")
    }
}

fn log(cmd: &Command) -> String {
    let command = format_program_and_args(cmd);
    tracing::debug!("Executing command `{command}`");
    command
}
