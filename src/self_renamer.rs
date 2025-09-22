use color_eyre::eyre::Result;
use std::{env::current_exe, fs, path::PathBuf};
use tracing::{debug, error, warn};

pub struct SelfRenamer {
    exe_path: PathBuf,
    temp_path: PathBuf,
}

impl SelfRenamer {
    pub fn create() -> Result<Self> {
        let tempdir = tempfile::tempdir()?;
        let mut temp_path = tempdir.path().join("topgrade.exe");
        let exe_path = current_exe()?;

        debug!(
            "Current exe in {:?}. Attempting to move it to {:?}",
            exe_path, temp_path
        );

        match fs::rename(&exe_path, &temp_path) {
            // cross-device error
            Err(e) if e.raw_os_error() == Some(17) => {
                debug!("Temporary directory is on a different device. Using the binary parent directory instead");

                let Some(parent_dir) = exe_path.parent() else {
                    return Err(color_eyre::eyre::Report::msg(
                        "Could not get parent directory of the current binary",
                    ));
                };

                let mut builder = tempfile::Builder::new();
                builder.prefix("topgrade").suffix(".exe");
                let temp_file = builder.tempfile_in(parent_dir)?;
                temp_path = temp_file.path().to_path_buf();

                // Delete the temporary file immediately to free up the name
                if let Err(e) = temp_file.close() {
                    warn!("Could not close temporary file: {}", e);
                }

                debug!("Moving current exe in {:?} to {:?}", exe_path, temp_path);
                fs::rename(&exe_path, &temp_path)
            }
            other => other,
        }?;

        Ok(SelfRenamer { exe_path, temp_path })
    }
}

impl Drop for SelfRenamer {
    fn drop(&mut self) {
        if self.exe_path.exists() {
            debug!("{:?} exists. Topgrade was probably upgraded", self.exe_path);
            if let Err(e) = self_replace::self_delete_at(&self.temp_path) {
                error!("Could not clean up temporarily renamed topgrade executable: {}", e);
            }
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
