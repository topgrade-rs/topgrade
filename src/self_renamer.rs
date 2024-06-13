#![cfg(windows)]

use color_eyre::eyre::Result;
use std::{env::current_exe, fs, path::PathBuf};
use tracing::{debug, error};

pub struct SelfRenamer {
    exe_path: PathBuf,
    temp_path: PathBuf,
}

impl SelfRenamer {
    pub fn create() -> Result<Self> {
        let tempdir = tempfile::tempdir()?;
        let temp_path = tempdir.path().join("topgrade.exe");
        let exe_path = current_exe()?;

        debug!("{} {:?}. {} {:?}", t!("Current exe in"), exe_path, t!("Moving it to"), temp_path);

        fs::rename(&exe_path, &temp_path)?;

        Ok(SelfRenamer { exe_path, temp_path })
    }
}

impl Drop for SelfRenamer {
    fn drop(&mut self) {
        if self.exe_path.exists() {
            debug!("{:?} {}", self.exe_path, t!("exists. Topgrade was probably upgraded"));
            return;
        }

        match fs::rename(&self.temp_path, &self.exe_path) {
            Ok(_) => debug!("{} {:?} {} {:?}", t!("Moved Topgrade back from"), self.temp_path, t!("to"), self.exe_path),
            Err(e) => error!(
                "{}",
                t!("Could not move Topgrade from {} back to {}: {}", temp_path=self.temp_path.display(), exe_path=self.exe_path.display(), error=e)
            ),
        }
    }
}
