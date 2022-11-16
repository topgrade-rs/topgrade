//! A stub for Ctrl + C handling.
use crate::ctrlc::interrupted::set_interrupted;
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
use winapi::um::consoleapi::SetConsoleCtrlHandler;
use winapi::um::wincon::CTRL_C_EVENT;

extern "system" fn handler(ctrl_type: DWORD) -> BOOL {
    match ctrl_type {
        CTRL_C_EVENT => {
            set_interrupted();
            TRUE
        }
        _ => FALSE,
    }
}

pub fn set_handler() {
    if 0 == unsafe { SetConsoleCtrlHandler(Some(handler), TRUE) } {
        tracing::error!("Cannot set a control C handler")
    }
}
