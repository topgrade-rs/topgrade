pub mod containers;
pub mod emacs;
pub mod generic;
pub mod git;
pub mod go;
pub mod kakoune;
pub mod node;
pub mod os;
pub mod powershell;
pub mod remote;
#[cfg(unix)]
pub mod tmux;
#[cfg(target_os = "linux")]
pub mod toolbx;
pub mod vim;
#[cfg(unix)]
pub mod zsh;

pub use self::os::*;
