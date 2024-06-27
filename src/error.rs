use std::{fmt::Display, process::ExitStatus};

use thiserror::Error;

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
            TopgradeError::ProcessFailed(process, exit_satus) => write!(f, "`{process}` failed: {exit_satus}"),
            TopgradeError::ProcessFailedWithOutput(process, exit_status, _) => {
                write!(f, "`{process}` failed: {exit_status}")
            }
            TopgradeError::UnknownLinuxDistribution => write!(f, "Unknown Linux Distribution"),
            TopgradeError::EmptyOSReleaseFile => write!(f, "File \"/etc/os-release\" does not exist or is empty"),
            TopgradeError::FailedGettingPackageManager => write!(f, "Failed getting the system package manager"),
        }
    }
}

#[derive(Error, Debug)]
pub struct StepFailed;

impl Display for StepFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "A step failed")
    }
}

#[derive(Error, Debug)]
pub struct DryRun();

impl Display for DryRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dry running")
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
        write!(f, "Topgrade Upgraded")
    }
}
