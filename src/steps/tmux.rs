use std::env;
use std::path::PathBuf;

use color_eyre::eyre::Result;
use etcetera::base_strategy::BaseStrategy;

use crate::command::CommandExt;
use crate::terminal::print_separator;
use crate::{HOME_DIR, XDG_DIRS};
use crate::{execution_context::ExecutionContext, utils::PathExt};

// update_plugins path is relative to the TPM path
const UPDATE_PLUGINS: &str = "bin/update_plugins";
// Default TPM path relative to the TMux config directory
const TPM_PATH: &str = "plugins/tpm";

pub fn run_tpm(ctx: &ExecutionContext) -> Result<()> {
    let tpm = match env::var("TMUX_PLUGIN_MANAGER_PATH") {
        // Use `$TMUX_PLUGIN_MANAGER_PATH` if set,
        Ok(var) => PathBuf::from(var).join(UPDATE_PLUGINS),
        Err(_) => {
            // otherwise, use the default XDG location `~/.config/tmux`
            #[cfg(unix)]
            let xdg_path = XDG_DIRS.config_dir().join("tmux").join(TPM_PATH).join(UPDATE_PLUGINS);
            #[cfg(windows)]
            let xdg_path = HOME_DIR.join(".config/tmux").join(TPM_PATH).join(UPDATE_PLUGINS);
            if xdg_path.exists() {
                xdg_path
            } else {
                // or fallback on the standard default location `~/.tmux`.
                HOME_DIR.join(".tmux").join(TPM_PATH).join(UPDATE_PLUGINS)
            }
        }
    }
    .require()?;

    print_separator("tmux plugins");

    ctx.execute(tpm).arg("all").status_checked()
}
