pub mod config;
pub mod ctrlc;
pub mod error;
// pub mod execution_context;
pub mod executor;
pub mod report;
// pub mod runner;

// pub mod steps;
pub mod terminal;
pub mod utils;

#[cfg(feature = "self-update")]
mod self_update;

#[cfg(windows)]
mod self_renamer;
