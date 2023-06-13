use std::process::ExitStatus;

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TopgradeError {
    #[error("`{0}` failed: {1}")]
    ProcessFailed(String, ExitStatus),

    #[error("`{0}` failed: {1}")]
    ProcessFailedWithOutput(String, ExitStatus, String),

    #[error("Unknown Linux Distribution")]
    #[cfg(target_os = "linux")]
    UnknownLinuxDistribution,

    #[error("Failed getting the system package manager")]
    #[cfg(target_os = "linux")]
    FailedGettingPackageManager,
}

#[derive(Error, Debug)]
#[error("A step failed")]
pub struct StepFailed;

#[derive(Error, Debug)]
#[error("Dry running")]
pub struct DryRun();

#[derive(Error, Debug)]
#[error("{0}")]
pub struct SkipStep(pub String);

#[cfg(all(windows, feature = "self-update"))]
#[derive(Error, Debug)]
#[error("Topgrade Upgraded")]
pub struct Upgraded(pub ExitStatus);
