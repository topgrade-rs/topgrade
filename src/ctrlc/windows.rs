//! A stub for Ctrl + C handling.
use crate::ctrlc::interrupted::set_interrupted;
use tracing::error;
use windows::Win32::System::Console::{CTRL_C_EVENT, SetConsoleCtrlHandler};
use windows::core::BOOL;

extern "system" fn handler(ctrl_type: u32) -> BOOL {
    match ctrl_type {
        CTRL_C_EVENT => {
            set_interrupted();
            true.into()
        }
        _ => false.into(),
    }
}

pub fn set_handler() {
    if let Err(e) = unsafe { SetConsoleCtrlHandler(Some(handler), true) } {
        error!("Cannot set a control C handler: {e}")
    }
}
