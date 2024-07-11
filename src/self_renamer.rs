use color_eyre::eyre::Result;
use rust_i18n::t;
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

        debug!(
            "{}",
            t!(
                "Current exe in {exe_path}. Moving it to {temp_path}",
                exe_path = format!("{exe_path:?}"),
                temp_path = format!("{temp_path:?}")
            )
        );

        fs::rename(&exe_path, &temp_path)?;

        Ok(SelfRenamer { exe_path, temp_path })
    }
}

impl Drop for SelfRenamer {
    fn drop(&mut self) {
        if self.exe_path.exists() {
            debug!(
                "{}",
                t!(
                    "{exe_path} exists. Topgrade was probably upgraded",
                    exe_path = format!("{:?}", self.exe_path)
                )
            );
            return;
        }

        match fs::rename(&self.temp_path, &self.exe_path) {
            Ok(_) => debug!(
                "{}",
                t!(
                    "Moved Topgrade back from {temp_path} to {exe_path}",
                    temp_path = format!("{:?}", &self.temp_path),
                    exe_path = format!("{:?}", &self.exe_path)
                )
            ),
            Err(e) => error!(
                "{}",
                t!(
                    "Could not move Topgrade from {temp_path} back to {exe_path}: {error}",
                    temp_path = self.temp_path.display(),
                    exe_path = self.exe_path.display(),
                    error = e
                )
            ),
        }
    }
}
