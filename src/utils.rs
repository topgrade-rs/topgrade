use crate::error::SkipStep;
use color_eyre::eyre::Result;

use std::env;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::{debug, error};

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

/* sys-info-rs
 *
 * Copyright (c) 2015 Siyu Wang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
#[cfg(target_family = "unix")]
pub fn hostname() -> Result<String> {
    use std::ffi;
    extern crate libc;

    unsafe {
        let buf_size = libc::sysconf(libc::_SC_HOST_NAME_MAX) as usize;
        let mut buf = Vec::<u8>::with_capacity(buf_size + 1);

        if libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf_size) < 0 {
            return Err(SkipStep(format!("Failed to get hostname: {}", std::io::Error::last_os_error())).into());
        }
        let hostname_len = libc::strnlen(buf.as_ptr() as *const libc::c_char, buf_size);
        buf.set_len(hostname_len);

        Ok(ffi::CString::new(buf).unwrap().into_string().unwrap())
    }
}

#[cfg(target_family = "windows")]
pub fn hostname() -> Result<String> {
    use crate::command::CommandExt;
    use std::process::Command;

    Command::new("hostname")
        .output_checked_utf8()
        .map_err(|err| SkipStep(format!("Failed to get hostname: {err}")).into())
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
