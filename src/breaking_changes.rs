//! Inform the users of the breaking changes introduced in this major release.
//!
//! Print the breaking changes and possibly a migration guide when:
//!     1. The Topgrade being executed is a new major release
//!     2. This is the first launch of that major release

use crate::terminal::print_separator;
#[cfg(windows)]
use crate::WINDOWS_DIRS;
#[cfg(unix)]
use crate::XDG_DIRS;
use color_eyre::eyre::Result;
use etcetera::base_strategy::BaseStrategy;
use rust_i18n::t;
use std::{
    env::var,
    fs::{read_to_string, OpenOptions},
    io::Write,
    path::PathBuf,
    str::FromStr,
};

/// Version string x.y.z
static VERSION_STR: &str = env!("CARGO_PKG_VERSION");

/// Version info
#[derive(Debug)]
pub(crate) struct Version {
    _major: u64,
    minor: u64,
    patch: u64,
}

impl FromStr for Version {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const NOT_SEMVER: &str = "Topgrade version is not semantic";
        const NOT_NUMBER: &str = "Topgrade version is not dot-separated numbers";

        let mut iter = s.split('.').take(3);
        let major = iter.next().expect(NOT_SEMVER).parse().expect(NOT_NUMBER);
        let minor = iter.next().expect(NOT_SEMVER).parse().expect(NOT_NUMBER);
        let patch = iter.next().expect(NOT_SEMVER).parse().expect(NOT_NUMBER);

        // They cannot be all 0s
        assert!(
            !(major == 0 && minor == 0 && patch == 0),
            "Version numbers cannot be all 0s"
        );

        Ok(Self {
            _major: major,
            minor,
            patch,
        })
    }
}

impl Version {
    /// True if this version is a new major release.
    pub(crate) fn is_new_major_release(&self) -> bool {
        // We have already checked that they cannot all be zeros, so `self.major`
        // is guaranteed to be non-zero.
        self.minor == 0 && self.patch == 0
    }
}

/// Topgrade's breaking changes
///
/// We store them in the compiled binary.
pub(crate) static BREAKINGCHANGES: &str = include_str!("../BREAKINGCHANGES.md");

/// Return platform's data directory.
fn data_dir() -> PathBuf {
    #[cfg(unix)]
    return XDG_DIRS.data_dir();

    #[cfg(windows)]
    return WINDOWS_DIRS.data_dir();
}

/// Return Topgrade's keep file path.
///
/// keep file is a file under the data directory containing a major version
/// number, it will be created on first run and is used to check if an execution
/// of Topgrade is the first run of a major release, for more details, see
/// `first_run_of_major_release()`.
fn keep_file_path() -> PathBuf {
    let keep_file = "topgrade_keep";
    data_dir().join(keep_file)
}

/// If environment variable `TOPGRADE_SKIP_BRKC_NOTIFY` is set to `true`, then
/// we won't notify the user of the breaking changes.
pub(crate) fn should_skip() -> bool {
    if let Ok(var) = var("TOPGRADE_SKIP_BRKC_NOTIFY") {
        return var.as_str() == "true";
    }

    false
}

/// True if this is the first execution of a major release.
pub(crate) fn first_run_of_major_release() -> Result<bool> {
    let version = VERSION_STR.parse::<Version>().expect("should be a valid version");
    let keep_file = keep_file_path();

    // disable this lint here as the current code has better readability
    #[allow(clippy::collapsible_if)]
    if version.is_new_major_release() {
        if !keep_file.exists() || read_to_string(&keep_file)? != VERSION_STR {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Print breaking changes to the user.
pub(crate) fn print_breaking_changes() {
    let header = format!(
        "{}",
        t!("Topgrade {version_str} Breaking Changes", version_str = VERSION_STR)
    );
    print_separator(header);
    let contents = if BREAKINGCHANGES.is_empty() {
        t!("No Breaking changes").to_string()
    } else {
        BREAKINGCHANGES.to_string()
    };
    println!("{contents}\n");
}

/// This function will be ONLY executed when the user has confirmed the breaking
/// changes, once confirmed, we write the keep file, which means the first run
/// of this major release is finished.
pub(crate) fn write_keep_file() -> Result<()> {
    std::fs::create_dir_all(data_dir())?;
    let keep_file = keep_file_path();

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(keep_file)?;
    let _ = file.write(VERSION_STR.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_new_major_release_works() {
        let first_major_release: Version = "1.0.0".parse().unwrap();
        let under_dev: Version = "0.1.0".parse().unwrap();

        assert!(first_major_release.is_new_major_release());
        assert!(!under_dev.is_new_major_release());
    }

    #[test]
    #[should_panic(expected = "Version numbers cannot be all 0s")]
    fn invalid_version() {
        let all_0 = "0.0.0";
        all_0.parse::<Version>().unwrap();
    }
}
