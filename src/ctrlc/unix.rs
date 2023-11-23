//! SIGINT handling in Unix systems.
use crate::ctrlc::interrupted::set_interrupted;
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};

/// Handle SIGINT. Set the interruption flag.
extern "C" fn handle_sigint(_: i32) {
    set_interrupted()
}

/// Set the necessary signal handlers.
/// The function panics on failure.
pub fn set_handler() {
    let sig_action = SigAction::new(SigHandler::Handler(handle_sigint), SaFlags::empty(), SigSet::empty());
    unsafe {
        sigaction(Signal::SIGINT, &sig_action).unwrap();
    }
}
