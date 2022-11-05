use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};

lazy_static! {
    /// A global variable telling whether the application has been interrupted.
    static ref INTERRUPTED: AtomicBool = AtomicBool::new(false);
}

/// Tells whether the program has been interrupted
pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Clears the interrupted flag
pub fn unset_interrupted() {
    debug_assert!(INTERRUPTED.load(Ordering::SeqCst));
    INTERRUPTED.store(false, Ordering::SeqCst)
}

pub fn set_interrupted() {
    INTERRUPTED.store(true, Ordering::SeqCst)
}
