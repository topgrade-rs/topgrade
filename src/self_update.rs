use std::env;
#[cfg(unix)]
use std::os::unix::process::CommandExt as _;
use std::process::Command;

use crate::config::Step;
use color_eyre::eyre::{bail, Result};
use self_update_crate::backends::github::Update;
use self_update_crate::update::UpdateStatus;

use super::terminal::*;
#[cfg(windows)]
use crate::error::Upgraded;

use crate::execution_context::ExecutionContext;

pub fn self_update(ctx: &ExecutionContext) -> Result<()> {
    print_separator("Self update");

    if ctx.run_type().dry() {
        println!("Would self-update");
        Ok(())
    } else {
        let assume_yes = ctx.config().yes(Step::SelfUpdate);
        let current_exe = env::current_exe();

        let target = self_update_crate::get_target();
        let result = Update::configure()
            .repo_owner("topgrade-rs")
            .repo_name("topgrade")
            .target(target)
            .bin_name(if cfg!(windows) { "topgrade.exe" } else { "topgrade" })
            .show_output(true)
            .show_download_progress(true)
            .current_version(self_update_crate::cargo_crate_version!())
            .no_confirm(assume_yes)
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
                print_info("Respawning...");
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
}
