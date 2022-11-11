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

        debug!("Current exe in {:?}. Moving it to {:?}", exe_path, temp_path);

        fs::rename(&exe_path, &temp_path)?;

        Ok(SelfRenamer { exe_path, temp_path })
    }
}

impl Drop for SelfRenamer {
    fn drop(&mut self) {
        if self.exe_path.exists() {
            debug!("{:?} exists. Topgrade was probably upgraded", self.exe_path);
            return;
        }

        match fs::rename(&self.temp_path, &self.exe_path) {
            Ok(_) => debug!("Moved Topgrade back from {:?} to {:?}", self.temp_path, self.exe_path),
            Err(e) => error!(
                "Could not move Topgrade from {} back to {}: {}",
                self.temp_path.display(),
                self.exe_path.display(),
                e
            ),
        }
    }
}
