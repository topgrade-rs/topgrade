use std::env;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::Result;
use rust_i18n::t;

use tracing::{debug, error};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload::{Handle, Layer};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, Registry};
use tracing_subscriber::{registry, EnvFilter};

use crate::command::CommandExt;
use crate::config::DEFAULT_LOG_LEVEL;
use crate::error::SkipStep;

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
            Err(SkipStep(format!(
                "{}",
                t!("Path {path} doesn't exist", path = format!("{:?}", self.as_ref()))
            ))
            .into())
        }
    }
}

pub fn which<T: AsRef<OsStr> + Debug>(binary_name: T) -> Option<PathBuf> {
    match which_crate::which(&binary_name) {
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

pub fn editor() -> Vec<String> {
    env::var("EDITOR")
        .unwrap_or_else(|_| String::from(if cfg!(windows) { "notepad" } else { "vi" }))
        .split_whitespace()
        .map(|s| s.to_owned())
        .collect()
}

pub fn require<T: AsRef<OsStr> + Debug>(binary_name: T) -> Result<PathBuf> {
    match which_crate::which(&binary_name) {
        Ok(path) => {
            debug!("Detected {:?} as {:?}", &path, &binary_name);
            Ok(path)
        }
        Err(e) => match e {
            which_crate::Error::CannotFindBinaryPath => Err(SkipStep(format!(
                "{}",
                t!(
                    "Cannot find {binary_name} in PATH",
                    binary_name = format!("{:?}", &binary_name)
                )
            ))
            .into()),
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

pub fn string_prepend_str(string: &mut String, s: &str) {
    let mut new_string = String::with_capacity(string.len() + s.len());
    new_string.push_str(s);
    new_string.push_str(string);
    *string = new_string;
}

#[cfg(target_family = "unix")]
pub fn hostname() -> Result<String> {
    match nix::unistd::gethostname() {
        Ok(os_str) => Ok(os_str
            .into_string()
            .map_err(|_| SkipStep(t!("Failed to get a UTF-8 encoded hostname").into()))?),
        Err(e) => Err(e.into()),
    }
}

#[cfg(target_family = "windows")]
pub fn hostname() -> Result<String> {
    Command::new("hostname")
        .output_checked_utf8()
        .map_err(|err| SkipStep(t!("Failed to get hostname: {err}", err = err).to_string()).into())
        .map(|output| output.stdout.trim().to_owned())
}

pub mod merge_strategies {
    use merge::Merge;

    use crate::config::Commands;

    /// Prepends right to left (both Option<Vec<T>>)
    pub fn vec_prepend_opt<T>(left: &mut Option<Vec<T>>, right: Option<Vec<T>>) {
        if let Some(left_vec) = left {
            if let Some(mut right_vec) = right {
                right_vec.append(left_vec);
                let _ = std::mem::replace(left, Some(right_vec));
            }
        } else {
            *left = right;
        }
    }

    /// Appends an Option<String> to another Option<String>
    pub fn string_append_opt(left: &mut Option<String>, right: Option<String>) {
        if let Some(left_str) = left {
            if let Some(right_str) = right {
                left_str.push(' ');
                left_str.push_str(&right_str);
            }
        } else {
            *left = right;
        }
    }

    pub fn inner_merge_opt<T>(left: &mut Option<T>, right: Option<T>)
    where
        T: Merge,
    {
        if let Some(ref mut left_inner) = left {
            if let Some(right_inner) = right {
                left_inner.merge(right_inner);
            }
        } else {
            *left = right;
        }
    }

    pub fn commands_merge_opt(left: &mut Option<Commands>, right: Option<Commands>) {
        if let Some(ref mut left_inner) = left {
            if let Some(right_inner) = right {
                left_inner.extend(right_inner);
            }
        } else {
            *left = right;
        }
    }
}

// Skip causes
// TODO: Put them in a better place when we have more of them
pub fn get_require_sudo_string() -> String {
    t!("Require sudo or counterpart but not found, skip").to_string()
}

/// Return `Err(SkipStep)` if `python` is a Python 2 or shim.
///
/// # Shim
/// On Windows, if you install `python` through `winget`, an actual `python`
/// is installed as well as a `python3` shim. Shim is invokable, but when you
/// execute it, the Microsoft App Store will be launched instead of a Python
/// shell.
///
/// We do this check through `python -V`, a shim will just give `Python` with
/// no version number.
pub fn check_is_python_2_or_shim(python: PathBuf) -> Result<PathBuf> {
    let output = Command::new(&python).arg("-V").output_checked_utf8()?;
    // "Python x.x.x\n"
    let stdout = output.stdout;
    // ["Python"] or ["Python", "x.x.x"], the newline char is trimmed.
    let mut split = stdout.split_whitespace();

    if let Some(version) = split.nth(1) {
        let major_version = version
            .split('.')
            .next()
            .expect("Should have a major version number")
            .parse::<u32>()
            .expect("Major version should be a valid number");
        if major_version == 2 {
            return Err(SkipStep(t!("{python} is a Python 2, skip.", python = python.display()).to_string()).into());
        }
    } else {
        // No version number, is a shim
        return Err(SkipStep(t!("{python} is a Python shim, skip.", python = python.display()).to_string()).into());
    }

    Ok(python)
}

/// Set up the tracing logger
///
/// # Return value
/// A reload handle will be returned so that we can change the log level at
/// runtime.
pub fn install_tracing(filter_directives: &str) -> Result<Handle<EnvFilter, Registry>> {
    let env_filter = EnvFilter::try_new(filter_directives)
        .or_else(|_| EnvFilter::try_from_default_env())
        .or_else(|_| EnvFilter::try_new(DEFAULT_LOG_LEVEL))?;

    let fmt_layer = fmt::layer().with_target(false).without_time();

    let (filter, reload_handle) = Layer::new(env_filter);

    registry().with(filter).with(fmt_layer).init();

    Ok(reload_handle)
}

/// Update the tracing logger with new `filter_directives`.
pub fn update_tracing(reload_handle: &Handle<EnvFilter, Registry>, filter_directives: &str) -> Result<()> {
    let new = EnvFilter::try_new(filter_directives)
        .or_else(|_| EnvFilter::try_from_default_env())
        .or_else(|_| EnvFilter::try_new(DEFAULT_LOG_LEVEL))?;
    reload_handle.modify(|old| *old = new)?;

    Ok(())
}

/// Set up the error handler crate
pub fn install_color_eyre() -> Result<()> {
    color_eyre::config::HookBuilder::new()
        // Don't display the backtrace reminder by default:
        //   Backtrace omitted. Run with RUST_BACKTRACE=1 environment variable to display it.
        //   Run with RUST_BACKTRACE=full to include source snippets.
        .display_env_section(false)
        // Display location information by default:
        //   Location:
        //      src/steps.rs:92
        .display_location_section(true)
        .install()
}
