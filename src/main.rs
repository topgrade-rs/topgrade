#![allow(clippy::cognitive_complexity)]

use std::env;
use std::env::home_dir;
use std::io;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;

use clap::CommandFactory;
use clap::{Parser, crate_version};
use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use crossterm::event::KeyCode;
#[cfg(windows)]
use etcetera::base_strategy::Windows;
#[cfg(unix)]
use etcetera::base_strategy::Xdg;
use rust_i18n::{i18n, t};
use std::sync::{LazyLock, OnceLock};
use tempfile::{TempDir, tempdir};
use tracing::debug;

use self::config::{CommandLineArgs, Config};
use self::error::StepFailed;
use self::runner::StepResult;
use self::steps::{remote::*, *};
use self::sudo::{Sudo, SudoCreateError, SudoKind};
use self::terminal::*;
use self::utils::{install_color_eyre, install_tracing, is_elevated, set_wsl_use_windows_path, update_tracing};

mod breaking_changes;
mod command;
mod config;
mod ctrlc;
mod error;
mod execution_context;
mod executor;
mod runner;
#[cfg(windows)]
mod self_renamer;
#[cfg(feature = "self-update")]
mod self_update;
mod step;
mod steps;
mod sudo;
mod terminal;
#[cfg(unix)]
mod tmux;
mod utils;

// Users without home directory are possible, but no-one has complained yet
pub(crate) static HOME_DIR: LazyLock<PathBuf> = LazyLock::new(|| home_dir().expect("No home directory"));
#[cfg(unix)]
pub(crate) static XDG_DIRS: LazyLock<Xdg> = LazyLock::new(|| Xdg::new().expect("No home directory"));
#[cfg(windows)]
pub(crate) static WINDOWS_DIRS: LazyLock<Windows> = LazyLock::new(|| Windows::new().expect("No home directory"));

// Init and load the i18n files
i18n!("locales", fallback = "en");

pub(crate) static OLD_CWD: OnceLock<PathBuf> = OnceLock::new();

struct TempCwd {
    #[allow(unused)]
    temp_dir: TempDir,
    old_cwd: PathBuf,
}

impl TempCwd {
    fn new() -> Result<Self> {
        let old_cwd = env::current_dir()?;
        let temp_dir = tempdir()?;
        env::set_current_dir(&temp_dir)?;
        Ok(Self { temp_dir, old_cwd })
    }
}

impl Drop for TempCwd {
    fn drop(&mut self) {
        env::set_current_dir(&self.old_cwd).expect("Restoring cwd failed");
    }
}

/// Whether `TOPGRADE_PREFIX` is the Toolbx child Topgrade.
///
/// Toolbx shares the host home and the host system bus, so a configured `run0`
/// would execute on the host (exit status 203 inside the container).
/// `src/steps/toolbx.rs` passes `TOPGRADE_PREFIX=Toolbx <name>` to `env`.
/// The name must be nonempty: SSH hosts, Distrobox, and any other prefix stay
/// unrestricted.
///
/// See: https://github.com/topgrade-rs/topgrade/issues/2373
fn is_toolbx_child(prefix: Option<&str>) -> bool {
    matches!(prefix.and_then(|prefix| prefix.strip_prefix("Toolbx ")), Some(name) if !name.is_empty())
}

/// Choose the privilege-escalation command.
///
/// Explicit kinds other than `run0` are kept. Inside Toolbx, configured `run0`
/// is treated as unset, so detection runs with `run0` removed. Elevated
/// execution with no kind, including that rewritten `run0`, stays `Null`.
fn select_sudo(configured: Option<SudoKind>, elevated: bool, prefix: Option<&str>) -> Result<Sudo, SudoCreateError> {
    let in_toolbx = is_toolbx_child(prefix);
    if in_toolbx {
        debug!("Toolbx child: excluding run0 from privilege escalation");
    }

    let configured = match configured {
        Some(SudoKind::Run0) if in_toolbx => None,
        other => other,
    };
    match configured {
        Some(kind) => Sudo::new(kind),
        None if elevated => Sudo::new(SudoKind::Null),
        // Toolbx exposes the host system bus, so `run0` would act on the host.
        None if in_toolbx => Sudo::detect(&[SudoKind::Run0]),
        None => Sudo::detect(&[]),
    }
}

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

    for (key, value) in opt.env_variables() {
        unsafe { env::set_var(key, value) };
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
    set_wsl_use_windows_path(config.wsl_use_windows_path())?;

    debug!("Version: {}", crate_version!());
    debug!("OS: {}", env!("TARGET"));
    debug!("{:?}", env::args());
    debug!("Binary path: {:?}", env::current_exe());
    debug!("self-update Feature Enabled: {:?}", cfg!(feature = "self-update"));
    debug!("Configuration: {:?}", config);

    if config.run_in_tmux() && env::var("TOPGRADE_INSIDE_TMUX").is_err() {
        #[cfg(unix)]
        {
            tmux::run_in_tmux(config.tmux_config()?)?;
            return Ok(());
        }
    }

    // Some steps (like mise or pi) have different behavior when ran in a project directory.
    //  Since Topgrade only handles global updates, run all commands in a temporary directory.
    let temp_cwd = TempCwd::new()?;
    OLD_CWD.set(temp_cwd.old_cwd.clone()).unwrap();

    let elevated = is_elevated();

    #[cfg(unix)]
    if !config.allow_root() && elevated {
        print_warning(t!(
            "Topgrade should not be run as root, it will run commands with sudo or equivalent where needed."
        ));
        if !prompt_yesno(&t!("Continue?"))? {
            exit(1)
        }
    }

    // The Toolbx child inherits the host config. Its prefix is what tells us
    // to keep `run0` off the host bus; other prefixes stay on the normal path.
    let prefix = env::var("TOPGRADE_PREFIX").ok();
    let sudo = select_sudo(config.sudo_command(), elevated, prefix.as_deref());
    debug!("Sudo: {:?}", sudo);

    let (sudo, sudo_err) = match sudo {
        Ok(sudo) => (Some(sudo), None),
        Err(e) => (None, Some(e)),
    };

    #[cfg(target_os = "linux")]
    let distribution = linux::Distribution::detect();

    let run_type = config.run_type();
    let ctx = execution_context::ExecutionContext::new(
        run_type,
        sudo,
        &config,
        #[cfg(target_os = "linux")]
        &distribution,
    );
    let mut runner = runner::Runner::new(&ctx);

    if !breaking_changes::should_skip() {
        breaking_changes::run()?;
    }

    step::Step::SelfUpdate.run(&mut runner, &ctx)?;

    #[cfg(windows)]
    let _self_rename = if config.self_rename() {
        Some(crate::self_renamer::SelfRenamer::create()?)
    } else {
        None
    };

    if let Some(sudo) = ctx.sudo()
        && (config.pre_sudo() || (config.sudo_loop() && sudo.can_refresh()))
    {
        sudo.elevate(&ctx)?;
    }

    // Held until `run()` returns — dropping would stop the background thread.
    let _sudo_loop_guard = spawn_sudo_loop(&ctx, &config)?;

    if let Some(commands) = config.pre_commands() {
        for (name, command) in commands {
            generic::run_custom_command(name, command, &ctx)?;
        }
    }

    for step in config.steps()? {
        match step.run(&mut runner, &ctx) {
            Ok(()) => (),
            Err(error)
                if error
                    .downcast_ref::<io::Error>()
                    .is_some_and(|e| e.kind() == io::ErrorKind::Interrupted) =>
            {
                println!();
                debug!("Interrupted (possibly with 'q' during retry prompt). Printing summary.");
                break;
            }
            Err(error) => return Err(error),
        }
    }

    let mut failed = false;

    let report = runner.report();
    if !report.is_empty() {
        print_separator(t!("Summary"));

        let mut skipped_missing_sudo = false;

        for (key, result) in report {
            if !failed && result.failed() {
                failed = true;
            }
            if let StepResult::SkippedMissingSudo = result {
                skipped_missing_sudo = true;
            }
            print_result(key, result);
        }

        if skipped_missing_sudo {
            print_warning(t!(
                "\nSome steps were skipped as sudo or equivalent could not be found."
            ));
            // Steps can only fail with SkippedMissingSudo if sudo is None,
            // therefore we must have a sudo_err
            match sudo_err.unwrap() {
                SudoCreateError::CannotFindBinary => {
                    #[cfg(unix)]
                    print_warning(t!(
                        "Install one of `sudo`, `doas`, `pkexec`, `run0` or `please` to run these steps."
                    ));

                    // If this Windows version supported Windows Sudo, the error would have been WinSudoDisabled
                    #[cfg(windows)]
                    print_warning(t!("Install gsudo to run these steps."));
                }
                #[cfg(windows)]
                SudoCreateError::WinSudoDisabled => {
                    print_warning(t!(
                        "Install gsudo or enable Windows Sudo to run these steps.\nFor Windows Sudo, the default 'In a new window' mode is not supported as it prevents Topgrade from waiting for commands to finish. Please configure it to use 'Inline' mode instead.\nGo to https://go.microsoft.com/fwlink/?linkid=2257346 to learn more."
                    ));
                }
                #[cfg(windows)]
                SudoCreateError::WinSudoNewWindowMode => {
                    print_warning(t!(
                        "Windows Sudo was found, but it is set to 'In a new window' mode, which prevents Topgrade from waiting for commands to finish. Please configure it to use 'Inline' mode instead.\nGo to https://go.microsoft.com/fwlink/?linkid=2257346 to learn more."
                    ));
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    if config.show_distribution_summary()
        && let Ok(distribution) = &distribution
    {
        distribution.show_summary();
    }

    if let Some(commands) = config.post_commands() {
        for (name, command) in commands {
            let result = generic::run_custom_command(name, command, &ctx);
            if !failed && result.is_err() {
                failed = true;
            }
        }
    }

    if config.keep_at_end() {
        print_info(t!("\n(R)eboot\n(P)oweroff\n(S)hell\n(Q)uit"));
        loop {
            match get_key() {
                Ok(KeyCode::Char('s' | 'S')) => {
                    run_shell().context("Failed to execute shell")?;
                }
                Ok(KeyCode::Char('r' | 'R')) => {
                    println!("{}", t!("Rebooting..."));
                    reboot(&ctx).context("Failed to reboot")?;
                }
                Ok(KeyCode::Char('p' | 'P')) => {
                    println!("{}", t!("Shutting down..."));
                    shutdown(&ctx).context("Failed to shut down")?;
                }
                Ok(KeyCode::Char('q' | 'Q')) => (),
                _ => {
                    continue;
                }
            }
            break;
        }
    }

    let should_notify = match config.notify_end() {
        config::NotifyEnd::Always => true,
        config::NotifyEnd::Never => false,
        config::NotifyEnd::OnFailure => failed,
    };

    if should_notify {
        notify_desktop(
            if failed {
                t!("Topgrade finished with errors")
            } else {
                t!("Topgrade finished successfully")
            },
            Some(Duration::from_secs(10)),
        );
    }

    if failed { Err(StepFailed.into()) } else { Ok(()) }
}

fn spawn_sudo_loop(
    ctx: &execution_context::ExecutionContext,
    config: &Config,
) -> Result<Option<std::sync::mpsc::Sender<()>>> {
    if !config.sudo_loop() {
        return Ok(None);
    }
    let sudo = match ctx.sudo().as_ref() {
        None => return Ok(None),
        Some(sudo) => sudo.clone(),
    };
    let run_type = ctx.run_type();
    let interval = Duration::from_secs(config.sudo_loop_interval().into());
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::Builder::new()
        .name("sudo-loop".into())
        .spawn(move || {
            loop {
                match rx.recv_timeout(interval) {
                    Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if let Err(err) = sudo.refresh(run_type) {
                            print_warning(format!("Failed to refresh sudo credentials: {err:?}"));
                        }
                    }
                }
            }
        })
        .wrap_err("failed to spawn sudo-loop thread")?;
    Ok(Some(tx))
}

fn main() {
    match run() {
        Ok(()) => {
            exit(0);
        }
        Err(error) => {
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

#[cfg(test)]
mod tests {
    use super::{is_toolbx_child, select_sudo};
    use crate::sudo::{SudoCreateError, SudoKind};

    /// Exact value `env` stores for `TOPGRADE_PREFIX=Toolbx fedora`.
    const GENERATED_TOOLBX_PREFIX: &str = "Toolbx fedora";

    #[test]
    fn toolbx_child_recognizes_generated_prefix_only() {
        assert!(is_toolbx_child(Some(GENERATED_TOOLBX_PREFIX)));
        assert!(is_toolbx_child(Some("Toolbx dev")));

        assert!(!is_toolbx_child(None));
        for prefix in [
            "",
            "example.com",
            "user@example.com",
            "Toolbx",
            "Toolbx ",
            "my Toolbx container",
            "Distrobox fedora",
        ] {
            assert!(!is_toolbx_child(Some(prefix)), "{prefix:?}");
        }
    }

    #[test]
    fn elevated_without_command_or_restricted_run0_is_null() {
        for prefix in [
            None,
            Some(""),
            Some("example.com"),
            Some(GENERATED_TOOLBX_PREFIX),
            Some("Toolbx fedora"),
        ] {
            assert_eq!(
                select_sudo(None, true, prefix).unwrap().kind(),
                SudoKind::Null,
                "{prefix:?}"
            );
        }

        // Configured run0 inside Toolbx follows the same elevated fallback as an absent setting.
        for prefix in [Some(GENERATED_TOOLBX_PREFIX), Some("Toolbx fedora")] {
            assert_eq!(
                select_sudo(Some(SudoKind::Run0), true, prefix).unwrap().kind(),
                SudoKind::Null,
                "{prefix:?}"
            );
        }
    }

    /// Lookup runs in a child so this process never mutates PATH. Fixtures are empty and never executed.
    #[cfg(unix)]
    #[test]
    fn selection_policy_excludes_run0_inside_toolbx() {
        if isolated_sudo_child() {
            selection_policy_child();
            mark_isolated_sudo_child();
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        for name in ["doas", "sudo", "pkexec", "run0", "please"] {
            write_executable(dir.path(), name);
        }
        let marker = dir.path().join("isolated-ok");
        spawn_isolated_sudo_case(dir.path(), &marker);
    }

    #[cfg(unix)]
    fn selection_policy_child() {
        let root = std::path::PathBuf::from(std::env::var_os("PATH").unwrap());
        let quoted = Some(GENERATED_TOOLBX_PREFIX);
        let unquoted = Some("Toolbx fedora");

        let assert_kind = |configured, elevated, prefix, expected: SudoKind| {
            let sudo = select_sudo(configured, elevated, prefix)
                .unwrap_or_else(|err| panic!("expected {expected:?}, got {err:?} for {prefix:?}"));
            assert_eq!(sudo.kind(), expected, "{prefix:?}");
            let path = sudo.path().unwrap();
            assert!(
                path.starts_with(&root),
                "selected {expected:?} outside the fixture PATH: {}",
                path.display()
            );
        };

        // Explicit non-run0 kinds stay explicit inside Toolbx, elevated or not.
        assert_kind(Some(SudoKind::Sudo), false, quoted, SudoKind::Sudo);
        assert_kind(Some(SudoKind::Sudo), true, quoted, SudoKind::Sudo);
        assert_kind(Some(SudoKind::Doas), false, quoted, SudoKind::Doas);
        assert_kind(Some(SudoKind::Please), false, unquoted, SudoKind::Please);

        // Outside Toolbx, configured run0 stays explicit even though doas is first on PATH.
        for prefix in [
            None,
            Some(""),
            Some("example.com"),
            Some("user@example.com"),
            Some("my Toolbx container"),
        ] {
            assert_kind(Some(SudoKind::Run0), false, prefix, SudoKind::Run0);
            assert_kind(Some(SudoKind::Run0), true, prefix, SudoKind::Run0);
        }

        // Inside Toolbx, configured run0 and an unset command both autodetect.
        // doas is the first eligible candidate, so this is not explicit run0.
        for prefix in [quoted, unquoted] {
            assert_kind(Some(SudoKind::Run0), false, prefix, SudoKind::Doas);
            assert_kind(None, false, prefix, SudoKind::Doas);
        }

        for name in ["doas", "sudo", "pkexec", "please"] {
            std::fs::remove_file(root.join(name)).unwrap();
        }

        // Only run0 remains. Host autodetection still accepts it; Toolbx must not escape to it.
        for prefix in [
            None,
            Some(""),
            Some("example.com"),
            Some("user@example.com"),
            Some("my Toolbx container"),
        ] {
            assert_kind(None, false, prefix, SudoKind::Run0);
            assert_kind(Some(SudoKind::Run0), false, prefix, SudoKind::Run0);
        }
        for prefix in [quoted, unquoted] {
            for configured in [Some(SudoKind::Run0), None] {
                assert!(
                    matches!(
                        select_sudo(configured, false, prefix),
                        Err(SudoCreateError::CannotFindBinary)
                    ),
                    "{configured:?} {prefix:?}"
                );
            }
        }
    }

    #[cfg(unix)]
    fn isolated_sudo_child() -> bool {
        std::env::var_os("TOPGRADE_ISOLATED_SUDO_CASE").is_some()
    }

    #[cfg(unix)]
    fn mark_isolated_sudo_child() {
        std::fs::write(std::env::var_os("TOPGRADE_ISOLATED_SUDO_MARKER").unwrap(), "ok").unwrap();
    }

    #[cfg(unix)]
    fn write_executable(dir: &std::path::Path, name: &str) {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, b"").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(unix)]
    #[expect(clippy::disallowed_methods)]
    fn spawn_isolated_sudo_case(path_dir: &std::path::Path, marker: &std::path::Path) {
        let current = std::thread::current();
        let test_name = current.name().expect("libtest names the test thread");
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test_name, "--test-threads=1"])
            .env("PATH", path_dir)
            .env("TOPGRADE_ISOLATED_SUDO_CASE", "1")
            .env("TOPGRADE_ISOLATED_SUDO_MARKER", marker)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success() && std::fs::read_to_string(marker).is_ok_and(|text| text == "ok"),
            "isolated `{test_name}` exited {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status
        );
    }
}
