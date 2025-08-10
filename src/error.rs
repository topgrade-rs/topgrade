use std::{fmt::Display, process::ExitStatus};

use rust_i18n::t;
use thiserror::Error;

use crate::sudo::SudoKind;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TopgradeError {
    ProcessFailed(String, ExitStatus),

    ProcessFailedWithOutput(String, ExitStatus, String),

    #[cfg(target_os = "linux")]
    UnknownLinuxDistribution,

    #[cfg(target_os = "linux")]
    EmptyOSReleaseFile,

    #[cfg(target_os = "linux")]
    FailedGettingPackageManager,
}

impl Display for TopgradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopgradeError::ProcessFailed(process, exit_status) => {
                write!(
                    f,
                    "{}",
                    t!(
                        "`{process}` failed: {exit_status}",
                        process = process,
                        exit_status = exit_status
                    )
                )
            }
            TopgradeError::ProcessFailedWithOutput(process, exit_status, output) => {
                write!(
                    f,
                    "{}",
                    t!(
                        "`{process}` failed: {exit_status} with {output}",
                        process = process,
                        exit_status = exit_status,
                        output = output
                    )
                )
            }
            #[cfg(target_os = "linux")]
            TopgradeError::UnknownLinuxDistribution => write!(f, "{}", t!("Unknown Linux Distribution")),
            #[cfg(target_os = "linux")]
            TopgradeError::EmptyOSReleaseFile => {
                write!(f, "{}", t!("File \"/etc/os-release\" does not exist or is empty"))
            }
            #[cfg(target_os = "linux")]
            TopgradeError::FailedGettingPackageManager => {
                write!(f, "{}", t!("Failed getting the system package manager"))
            }
        }
    }
}

#[derive(Error, Debug)]
pub struct StepFailed;

impl Display for StepFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", t!("A step failed"))
    }
}

#[derive(Error, Debug)]
pub struct UnsupportedSudo<'a> {
    pub sudo_kind: SudoKind,
    pub option: &'a str,
}

impl Display for UnsupportedSudo<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            t!(
                "{sudo_kind} does not support the {option} option",
                sudo_kind = self.sudo_kind,
                option = self.option
            )
        )
    }
}

#[derive(Error, Debug)]
pub struct DryRun();

impl Display for DryRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", t!("Dry running"))
    }
}

#[derive(Error, Debug)]
pub struct SkipStep(pub String);

impl Display for SkipStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(all(windows, feature = "self-update"))]
#[derive(Error, Debug)]
pub struct Upgraded(pub ExitStatus);

#[cfg(all(windows, feature = "self-update"))]
impl Display for Upgraded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", t!("Topgrade Upgraded"))
    }
}
