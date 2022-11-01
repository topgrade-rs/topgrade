#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "linux")]
mod archlinux;
#[cfg(target_os = "dragonfly")]
pub mod dragonfly;
#[cfg(target_os = "freebsd")]
pub mod freebsd;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "openbsd")]
pub mod openbsd;
#[cfg(unix)]
pub mod unix;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(windows)]
pub use windows::reboot;

#[cfg(unix)]
pub use unix::reboot;
