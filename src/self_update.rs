use std::env;
#[cfg(unix)]
use std::os::unix::process::CommandExt as _;
use std::process::Command;

use color_eyre::eyre::{bail, Result};
use self_update_crate::backends::github::Update;
use self_update_crate::update::UpdateStatus;

use super::terminal::*;
#[cfg(windows)]
use crate::error::Upgraded;

pub fn self_update() -> Result<()> {
    print_separator("Self update");
    let current_exe = env::current_exe();

    let target = self_update_crate::get_target();
    let result = Update::configure()
        .repo_owner("topgrade-rs")
        .repo_name("topgrade")
        .target(target)
        .bin_name(if cfg!(windows) { "topgrade.exe" } else { "topgrade" })
        .show_output(false)
        .show_download_progress(true)
        .current_version(self_update_crate::cargo_crate_version!())
        .no_confirm(true)
        .build()?
        .update_extended()?;

    if let UpdateStatus::Updated(release) = &result {
        println!("\nTopgrade upgraded to {}:\n", release.version);
        if let Some(body) = &release.body {
            println!("{body}");
        }
    } else {
        println!("Topgrade is up-to-date");
    }

    {
        if result.updated() {
            print_warning("Respawning...");
            let mut command = Command::new(current_exe?);
            command.args(env::args().skip(1)).env("TOPGRADE_NO_SELF_UPGRADE", "");

            #[cfg(unix)]
            {
                let err = command.exec();
                bail!(err);
            }

            #[cfg(windows)]
            {
                #[allow(clippy::disallowed_methods)]
                let status = command.status()?;
                bail!(Upgraded(status));
            }
        }
    }

    Ok(())
}
