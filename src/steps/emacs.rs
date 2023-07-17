#[cfg(windows)]
use std::env;
use std::path::{Path, PathBuf};

use color_eyre::eyre::Result;
use etcetera::base_strategy::BaseStrategy;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::{require, require_option, PathExt};
use crate::Step;

const EMACS_UPGRADE: &str = include_str!("emacs.el");
#[cfg(windows)]
const DOOM_PATH: &str = "bin/doom.cmd";
#[cfg(unix)]
const DOOM_PATH: &str = "bin/doom";

pub struct Emacs {
    directory: Option<PathBuf>,
    doom: Option<PathBuf>,
}

impl Emacs {
    fn directory_path() -> Option<PathBuf> {
        #[cfg(unix)]
        return {
            let emacs_xdg_dir = crate::XDG_DIRS.config_dir().join("emacs").if_exists();
            crate::HOME_DIR.join(".emacs.d").if_exists().or(emacs_xdg_dir)
        };

        #[cfg(windows)]
        return env::var("HOME")
            .ok()
            .and_then(|home| {
                PathBuf::from(&home)
                    .join(".emacs.d")
                    .if_exists()
                    .or_else(|| PathBuf::from(&home).join(".config\\emacs").if_exists())
            })
            .or_else(|| crate::WINDOWS_DIRS.data_dir().join(".emacs.d").if_exists());
    }

    pub fn new() -> Self {
        let directory = Emacs::directory_path();
        let doom = directory.as_ref().and_then(|d| d.join(DOOM_PATH).if_exists());
        Self { directory, doom }
    }

    pub fn is_doom(&self) -> bool {
        self.doom.is_some()
    }

    pub fn directory(&self) -> Option<&PathBuf> {
        self.directory.as_ref()
    }

    fn update_doom(doom: &Path, ctx: &ExecutionContext) -> Result<()> {
        print_separator("Doom Emacs");

        let mut command = ctx.run_type().execute(doom);
        if ctx.config().yes(Step::Emacs) {
            command.arg("--force");
        }

        command.args(["upgrade"]);

        command.status_checked()
    }

    pub fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        let emacs = require("emacs")?;
        if let Some(doom) = &self.doom {
            Emacs::update_doom(doom, ctx)?;
        }
        let init_file = require_option(self.directory.as_ref(), String::from("Emacs directory does not exist"))?
            .join("init.el")
            .require()?;

        print_separator("Emacs");

        let mut command = ctx.run_type().execute(emacs);

        command
            .args(["--batch", "--debug-init", "-l"])
            .arg(init_file)
            .arg("--eval");

        #[cfg(unix)]
        command.arg(
            EMACS_UPGRADE
                .chars()
                .map(|c| if c.is_whitespace() { '\u{00a0}' } else { c })
                .collect::<String>(),
        );

        #[cfg(not(unix))]
        command.arg(EMACS_UPGRADE);

        command.status_checked()
    }
}
