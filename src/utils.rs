use crate::error::{SkipStep, TopgradeError};
use anyhow::Result;

use log::{debug, error};

#[cfg(not(test))]
use std::env;

use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
#[cfg(not(test))]
use which_crate::which as which_crate_which;

pub trait Check {
    fn check(self) -> Result<()>;
}

impl Check for Output {
    fn check(self) -> Result<()> {
        self.status.check()
    }
}

pub trait CheckWithCodes {
    fn check_with_codes(self, codes: &[i32]) -> Result<()>;
}

// Anything that implements CheckWithCodes also implements check
// if check_with_codes is given an empty array of codes to check
impl<T: CheckWithCodes> Check for T {
    fn check(self) -> Result<()> {
        self.check_with_codes(&[])
    }
}

impl CheckWithCodes for ExitStatus {
    fn check_with_codes(self, codes: &[i32]) -> Result<()> {
        // Set the default to be -1 because the option represents a signal termination
        let code = self.code().unwrap_or(-1);
        if self.success() || codes.contains(&code) {
            Ok(())
        } else {
            Err(TopgradeError::ProcessFailed(self).into())
        }
    }
}

pub trait PathExt
where
    Self: Sized,
{
    fn if_exists(self) -> Option<Self>;
    fn is_descendant_of(&self, ancestor: &Path) -> bool;

    /// Returns the path if it exists or ErrorKind::SkipStep otherwise
    fn require(self) -> Result<Self>;
}

impl<T> PathExt for T
where
    T: AsRef<Path>,
{
    fn if_exists(self) -> Option<Self> {
        if self.as_ref().exists() {
            debug!("Path {:?} exists", self.as_ref());
            Some(self)
        } else {
            debug!("Path {:?} doesn't exist", self.as_ref());
            None
        }
    }

    fn is_descendant_of(&self, ancestor: &Path) -> bool {
        self.as_ref().iter().zip(ancestor.iter()).all(|(a, b)| a == b)
    }

    fn require(self) -> Result<Self> {
        if self.as_ref().exists() {
            debug!("Path {:?} exists", self.as_ref());
            Ok(self)
        } else {
            Err(SkipStep(format!("Path {:?} doesn't exist", self.as_ref())).into())
        }
    }
}

pub fn which<T: AsRef<OsStr> + Debug>(binary_name: T) -> Option<PathBuf> {
    match which_crate_which(&binary_name) {
        Ok(path) => {
            debug!("Detected {:?} as {:?}", &path, &binary_name);
            Some(path)
        }
        Err(e) => {
            match e {
                which_crate::Error::CannotFindBinaryPath => {
                    debug!("Cannot find {:?}", &binary_name);
                }
                _ => {
                    error!("Detecting {:?} failed: {}", &binary_name, e);
                }
            }

            None
        }
    }
}

pub fn sudo() -> Option<PathBuf> {
    which("doas")
        .or_else(|| which("sudo"))
        .or_else(|| which("gsudo"))
        .or_else(|| which("pkexec"))
}

pub fn editor() -> Vec<String> {
    #[cfg(not(test))]
    let editor = env::var("EDITOR")
        .unwrap_or_else(|_| String::from(if cfg!(windows) { "notepad" } else { "vi" }))
        .split_whitespace()
        .map(|s| s.to_owned())
        .collect();
    #[cfg(test)]
    let editor = String::from("vi").split_whitespace().map(|s| s.to_owned()).collect();

    return editor;
}

pub fn require<T: AsRef<OsStr> + Debug>(binary_name: T) -> Result<PathBuf> {
    match which_crate_which(&binary_name) {
        Ok(path) => {
            debug!("Detected {:?} as {:?}", &path, &binary_name);
            Ok(path)
        }
        Err(e) => match e {
            which_crate::Error::CannotFindBinaryPath => {
                Err(SkipStep(format!("Cannot find {:?} in PATH", &binary_name)).into())
            }
            _ => {
                panic!("Detecting {:?} failed: {}", &binary_name, e);
            }
        },
    }
}

#[allow(dead_code)]
pub fn require_option<T>(option: Option<T>, cause: String) -> Result<T> {
    if let Some(value) = option {
        Ok(value)
    } else {
        Err(SkipStep(cause).into())
    }
}

#[cfg(test)]
pub fn which_crate_which<T: AsRef<OsStr>>(binary_name: T) -> Result<PathBuf, which_crate::Error> {
    let bin_name = binary_name.as_ref().to_str().unwrap();
    let mut path = PathBuf::new();
    path.push("/bin/");
    path.push(bin_name);

    return Ok(path);
}

#[cfg(test)]
pub fn create_test_path(binary_name: &str) -> PathBuf {
    let mut path = PathBuf::new();
    path.push("/bin/");
    path.push(binary_name);
    return path;
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_require() {
        let ls_path = require("test_require").unwrap();
        assert_eq!(ls_path, create_test_path("test_require"))
    }

    #[test]
    fn test_which() {
        let path = which("test_which").unwrap();
        assert_eq!(path, create_test_path("test_which"))
    }

    #[test]
    fn test_sudo() {
        let path = sudo().unwrap();
        assert_eq!(path, create_test_path("doas"))
    }

    #[test]
    fn test_editor() {
        let editor = editor();
        let mut challenge = Vec::<String>::new();
        challenge.push(String::from("vi"));
        assert_eq!(editor, challenge)
    }
}
