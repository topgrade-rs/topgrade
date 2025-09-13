#![allow(clippy::cognitive_complexity)]

use std::env;
use std::io;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;

use crate::breaking_changes::{first_run_of_major_release, print_breaking_changes, should_skip, write_keep_file};
use clap::CommandFactory;
use clap::{crate_version, Parser};
use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use console::Key;
use etcetera::base_strategy::BaseStrategy;
#[cfg(windows)]
use etcetera::base_strategy::Windows;
#[cfg(unix)]
use etcetera::base_strategy::Xdg;
use rust_i18n::{i18n, t};
use std::sync::LazyLock;
use tracing::debug;

use self::config::{CommandLineArgs, Config};
use self::error::StepFailed;
#[cfg(all(windows, feature = "self-update"))]
use self::error::Upgraded;
#[allow(clippy::wildcard_imports)]
use self::steps::{remote::*, *};
#[allow(clippy::wildcard_imports)]
use self::terminal::*;
use self::utils::{install_color_eyre, install_tracing, update_tracing};

mod breaking_changes;
mod command;
mod config;
mod ctrlc;
mod error;
mod execution_context;
mod executor;
mod report;
mod runner;
#[cfg(windows)]
mod self_renamer;
#[cfg(feature = "self-update")]
mod self_update;
mod step;
mod steps;
mod sudo;
mod terminal;
mod utils;

pub(crate) static HOME_DIR: LazyLock<PathBuf> = LazyLock::new(|| home::home_dir().expect("No home directory"));
#[cfg(unix)]
pub(crate) static XDG_DIRS: LazyLock<Xdg> = LazyLock::new(|| Xdg::new().expect("No home directory"));

#[cfg(windows)]
pub(crate) static WINDOWS_DIRS: LazyLock<Windows> = LazyLock::new(|| Windows::new().expect("No home directory"));

// Init and load the i18n files
i18n!("locales", fallback = "en");

#[allow(clippy::too_many_lines)]
fn run() -> Result<()> {
    install_color_eyre()?;
    ctrlc::set_handler();

    let opt = CommandLineArgs::parse();
    // Set up the logger with the filter directives from:
    //     1. CLI option `--log-filter`
    //     2. `debug` if the `--verbose` option is present
    // We do this because we need our logger to work while loading the
    // configuration file.
    //
    // When the configuration file is loaded, update the logger with the full
    // filter directives.
    //
    // For more info, see the comments in `CommandLineArgs::tracing_filter_directives()`
    // and `Config::tracing_filter_directives()`.
    let reload_handle = install_tracing(&opt.tracing_filter_directives())?;

    // Get current system locale and set it as the default locale
    let system_locale = sys_locale::get_locale().unwrap_or("en".to_string());
    rust_i18n::set_locale(&system_locale);
    debug!("Current system locale is {system_locale}");

    if let Some(shell) = opt.gen_completion {
        let cmd = &mut CommandLineArgs::command();
        clap_complete::generate(shell, cmd, clap::crate_name!(), &mut io::stdout());
        return Ok(());
    }

    if opt.gen_manpage {
        let man = clap_mangen::Man::new(CommandLineArgs::command());
        man.render(&mut io::stdout())?;
        return Ok(());
    }

    for env in opt.env_variables() {
        let mut split = env.split('=');
        let var = split.next().unwrap();
        let value = split.next().unwrap();
        env::set_var(var, value);
    }

    if opt.edit_config() {
        Config::edit()?;
        return Ok(());
    };

    if opt.show_config_reference() {
        print!("{}", config::EXAMPLE_CONFIG);
        return Ok(());
    }

    let config = Config::load(opt)?;
    // Update the logger with the full filter directives.
    update_tracing(&reload_handle, &config.tracing_filter_directives())?;
    set_title(config.set_title());
    display_time(config.display_time());
    set_desktop_notifications(config.notify_each_step());

    debug!("Version: {}", crate_version!());
    debug!("OS: {}", env!("TARGET"));
    debug!("{:?}", env::args());
    debug!("Binary path: {:?}", std::env::current_exe());
    debug!("self-update Feature Enabled: {:?}", cfg!(feature = "self-update"));
    debug!("Configuration: {:?}", config);

    if config.run_in_tmux() && env::var("TOPGRADE_INSIDE_TMUX").is_err() {
        #[cfg(unix)]
        {
            tmux::run_in_tmux(config.tmux_config()?)?;
            return Ok(());
        }
    }

    #[cfg(target_os = "linux")]
    let distribution = linux::Distribution::detect();

    let sudo = config.sudo_command().map_or_else(sudo::Sudo::detect, sudo::Sudo::new);
    let run_type = execution_context::RunType::new(config.dry_run());
    let ctx = execution_context::ExecutionContext::new(
        run_type,
        sudo,
        &config,
        #[cfg(target_os = "linux")]
        &distribution,
    );
    let mut runner = runner::Runner::new(&ctx);

    // If
    //
    // 1. the breaking changes notification shouldn't be skipped
    // 2. this is the first execution of a major release
    //
    // inform user of breaking changes
    if !should_skip() && first_run_of_major_release()? {
        print_breaking_changes();

        if prompt_yesno("Confirmed?")? {
            write_keep_file()?;
        } else {
            exit(1);
        }
    }

    // Self-Update step, this will execute only if:
    // 1. the `self-update` feature is enabled
    // 2. it is not disabled from configuration (env var/CLI opt/file)
    #[cfg(feature = "self-update")]
    {
        let should_self_update = env::var("TOPGRADE_NO_SELF_UPGRADE").is_err() && !config.no_self_update();

        if should_self_update {
            runner.execute(step::Step::SelfUpdate, "Self Update", || self_update::self_update(&ctx))?;
        }
    }

    #[cfg(windows)]
    let _self_rename = if config.self_rename() {
        Some(crate::self_renamer::SelfRenamer::create()?)
    } else {
        None
    };

    if let Some(commands) = config.pre_commands() {
        for (name, command) in commands {
            generic::run_custom_command(name, command, &ctx)?;
        }
    }

    if config.pre_sudo() {
        if let Some(sudo) = ctx.sudo() {
            sudo.elevate(&ctx)?;
        }
    }

    for step in step::default_steps() {
        step.run(&mut runner, &ctx)?
    }

    if !runner.report().data().is_empty() {
        print_separator(t!("Summary"));

        for (key, result) in runner.report().data() {
            print_result(key, result);
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(distribution) = &distribution {
                distribution.show_summary();
            }
        }
    }

    let mut post_command_failed = false;
    if let Some(commands) = config.post_commands() {
        for (name, command) in commands {
            if generic::run_custom_command(name, command, &ctx).is_err() {
                post_command_failed = true;
            }
        }
    }

    if config.keep_at_end() {
        print_info(t!("\n(R)eboot\n(S)hell\n(Q)uit"));
        loop {
            match get_key() {
                Ok(Key::Char('s' | 'S')) => {
                    run_shell().context("Failed to execute shell")?;
                }
                Ok(Key::Char('r' | 'R')) => {
                    println!("{}", t!("Rebooting..."));
                    reboot(&ctx).context("Failed to reboot")?;
                }
                Ok(Key::Char('q' | 'Q')) => (),
                _ => {
                    continue;
                }
            }
            break;
        }
    }

    let failed = post_command_failed || runner.report().data().iter().any(|(_, result)| result.failed());

    if !config.skip_notify() {
        notify_desktop(
            if failed {
                t!("Topgrade finished with errors")
            } else {
                t!("Topgrade finished successfully")
            },
            Some(Duration::from_secs(10)),
        );
    }

    if failed {
        Err(StepFailed.into())
    } else {
        Ok(())
    }
}

fn main() {
    match run() {
        Ok(()) => {
            exit(0);
        }
        Err(error) => {
            #[cfg(all(windows, feature = "self-update"))]
            {
                if let Some(Upgraded(status)) = error.downcast_ref::<Upgraded>() {
                    exit(status.code().unwrap());
                }
            }

            let skip_print = (error.downcast_ref::<StepFailed>().is_some())
                || (error
                    .downcast_ref::<io::Error>()
                    .filter(|io_error| io_error.kind() == io::ErrorKind::Interrupted)
                    .is_some());

            if !skip_print {
                // The `Debug` implementation of `eyre::Result` prints a multi-line
                // error message that includes all the 'causes' added with
                // `.with_context(...)` calls.
                println!("{}", t!("Error: {error}", error = format!("{:?}", error)));
            }
            exit(1);
        }
    }
}
