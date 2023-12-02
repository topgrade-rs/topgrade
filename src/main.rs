#![allow(clippy::cognitive_complexity)]

use std::env;
use std::io;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;

use clap::CommandFactory;
use clap::{crate_version, Parser};
use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use console::Key;
use once_cell::sync::Lazy;
use tracing::debug;

use self::config::{CommandLineArgs, Config, Step};
use self::error::StepFailed;
#[cfg(all(windows, feature = "self-update"))]
use self::error::Upgraded;
use self::steps::{remote::*, *};
use self::terminal::*;

use self::utils::{install_color_eyre, install_tracing, update_tracing};

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
mod steps;
mod sudo;
mod terminal;
mod utils;

pub(crate) static HOME_DIR: Lazy<PathBuf> = Lazy::new(|| dirs::home_dir().expect("No home directory"));
pub(crate) static CONFIG_DIR: Lazy<PathBuf> = Lazy::new(|| dirs::config_dir().expect("No config directory"));
#[cfg(windows)]
pub(crate) static CACHE_DIR: Lazy<PathBuf> = Lazy::new(|| dirs::cache_dir().expect("No config directory"));
#[cfg(windows)]
pub(crate) static DATA_DIR: Lazy<PathBuf> = Lazy::new(|| dirs::data_dir().expect("No config directory"));

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
        let mut splitted = env.split('=');
        let var = splitted.next().unwrap();
        let value = splitted.next().unwrap();
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
    debug!("{:?}", std::env::args());
    debug!("Binary path: {:?}", std::env::current_exe());
    debug!("self-update Feature Enabled: {:?}", cfg!(feature = "self-update"));
    debug!("Configuration: {:?}", config);

    if config.run_in_tmux() && env::var("TOPGRADE_INSIDE_TMUX").is_err() {
        #[cfg(unix)]
        {
            tmux::run_in_tmux(config.tmux_arguments()?)?;
            return Ok(());
        }
    }

    let git = git::Git::new();
    let mut git_repos = git::Repositories::new(&git);
    let powershell = powershell::Powershell::new();
    let should_run_powershell = powershell.profile().is_some() && config.should_run(Step::Powershell);
    let emacs = emacs::Emacs::new();
    #[cfg(target_os = "linux")]
    let distribution = linux::Distribution::detect();

    let sudo = config.sudo_command().map_or_else(sudo::Sudo::detect, sudo::Sudo::new);
    let run_type = executor::RunType::new(config.dry_run());
    let ctx = execution_context::ExecutionContext::new(run_type, sudo, &git, &config);
    let mut runner = runner::Runner::new(&ctx);

    // Self-Update step, this will execute only if:
    // 1. the `self-update` feature is enabled
    // 2. it is not disabled from configuration (env var/CLI opt/file)
    #[cfg(feature = "self-update")]
    {
        let should_self_update = env::var("TOPGRADE_NO_SELF_UPGRADE").is_err() && !config.no_self_update();

        if should_self_update {
            runner.execute(Step::SelfUpdate, "Self Update", || self_update::self_update(&ctx))?;
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

    if let Some(topgrades) = config.remote_topgrades() {
        for remote_topgrade in topgrades.iter().filter(|t| config.should_execute_remote(t)) {
            runner.execute(Step::Remotes, format!("Remote ({remote_topgrade})"), || {
                ssh::ssh_step(&ctx, remote_topgrade)
            })?;
        }
    }

    #[cfg(windows)]
    {
        runner.execute(Step::Wsl, "WSL", || windows::run_wsl_topgrade(&ctx))?;
        runner.execute(Step::WslUpdate, "WSL", || windows::update_wsl(&ctx))?;
        runner.execute(Step::Chocolatey, "Chocolatey", || windows::run_chocolatey(&ctx))?;
        runner.execute(Step::Scoop, "Scoop", || windows::run_scoop(&ctx))?;
        runner.execute(Step::Winget, "Winget", || windows::run_winget(&ctx))?;
        runner.execute(Step::System, "Windows update", || windows::windows_update(&ctx))?;
    }

    #[cfg(target_os = "linux")]
    {
        // NOTE: Due to breaking `nu` updates, `packer.nu` needs to be updated before `nu` get updated
        // by other package managers.
        runner.execute(Step::Shell, "packer.nu", || linux::run_packer_nu(&ctx))?;

        match &distribution {
            Ok(distribution) => {
                runner.execute(Step::System, "System update", || distribution.upgrade(&ctx))?;
            }
            Err(e) => {
                println!("Error detecting current distribution: {e}");
            }
        }
        runner.execute(Step::ConfigUpdate, "config-update", || linux::run_config_update(&ctx))?;

        runner.execute(Step::AM, "am", || linux::run_am(&ctx))?;
        runner.execute(Step::AppMan, "appman", || linux::run_appman(&ctx))?;
        runner.execute(Step::DebGet, "deb-get", || linux::run_deb_get(&ctx))?;
        runner.execute(Step::Toolbx, "toolbx", || toolbx::run_toolbx(&ctx))?;
        runner.execute(Step::Snap, "snap", || linux::run_snap(&ctx))?;
        runner.execute(Step::Pacstall, "pacstall", || linux::run_pacstall(&ctx))?;
        runner.execute(Step::Pacdef, "pacdef", || linux::run_pacdef(&ctx))?;
        runner.execute(Step::Protonup, "protonup", || linux::run_protonup_update(&ctx))?;
        runner.execute(Step::Distrobox, "distrobox", || linux::run_distrobox_update(&ctx))?;
        runner.execute(Step::DkpPacman, "dkp-pacman", || linux::run_dkp_pacman_update(&ctx))?;
        runner.execute(Step::System, "pihole", || linux::run_pihole_update(&ctx))?;
        runner.execute(Step::Firmware, "Firmware upgrades", || linux::run_fwupdmgr(&ctx))?;
        runner.execute(Step::Restarts, "Restarts", || linux::run_needrestart(&ctx))?;

        runner.execute(Step::Flatpak, "Flatpak", || linux::run_flatpak(&ctx))?;
        runner.execute(Step::BrewFormula, "Brew", || {
            unix::run_brew_formula(&ctx, unix::BrewVariant::Path)
        })?;
        runner.execute(Step::Lure, "LURE", || linux::run_lure_update(&ctx))?;
    }

    #[cfg(target_os = "macos")]
    {
        runner.execute(Step::BrewFormula, "Brew (ARM)", || {
            unix::run_brew_formula(&ctx, unix::BrewVariant::MacArm)
        })?;
        runner.execute(Step::BrewFormula, "Brew (Intel)", || {
            unix::run_brew_formula(&ctx, unix::BrewVariant::MacIntel)
        })?;
        runner.execute(Step::BrewFormula, "Brew", || {
            unix::run_brew_formula(&ctx, unix::BrewVariant::Path)
        })?;
        runner.execute(Step::BrewCask, "Brew Cask (ARM)", || {
            unix::run_brew_cask(&ctx, unix::BrewVariant::MacArm)
        })?;
        runner.execute(Step::BrewCask, "Brew Cask (Intel)", || {
            unix::run_brew_cask(&ctx, unix::BrewVariant::MacIntel)
        })?;
        runner.execute(Step::BrewCask, "Brew Cask", || {
            unix::run_brew_cask(&ctx, unix::BrewVariant::Path)
        })?;
        runner.execute(Step::Macports, "MacPorts", || macos::run_macports(&ctx))?;
        runner.execute(Step::Sparkle, "Sparkle", || macos::run_sparkle(&ctx))?;
        runner.execute(Step::Mas, "App Store", || macos::run_mas(&ctx))?;
        runner.execute(Step::System, "System upgrade", || macos::upgrade_macos(&ctx))?;
    }

    #[cfg(target_os = "dragonfly")]
    {
        runner.execute(Step::Pkg, "DragonFly BSD Packages", || {
            dragonfly::upgrade_packages(&ctx)
        })?;
        dragonfly::audit_packages(&ctx)?;
    }

    #[cfg(target_os = "freebsd")]
    {
        runner.execute(Step::Pkg, "FreeBSD Packages", || freebsd::upgrade_packages(&ctx))?;
        runner.execute(Step::System, "FreeBSD Upgrade", || freebsd::upgrade_freebsd(&ctx))?;
        freebsd::audit_packages(&ctx)?;
    }

    #[cfg(target_os = "openbsd")]
    {
        runner.execute(Step::Pkg, "OpenBSD Packages", || openbsd::upgrade_packages(&ctx))?;
        runner.execute(Step::System, "OpenBSD Upgrade", || openbsd::upgrade_openbsd(&ctx))?;
    }

    #[cfg(target_os = "android")]
    {
        runner.execute(Step::Pkg, "Termux Packages", || android::upgrade_packages(&ctx))?;
    }

    #[cfg(unix)]
    {
        runner.execute(Step::Yadm, "yadm", || unix::run_yadm(&ctx))?;
        runner.execute(Step::Nix, "nix", || unix::run_nix(&ctx))?;
        runner.execute(Step::Guix, "guix", || unix::run_guix(&ctx))?;
        runner.execute(Step::HomeManager, "home-manager", || unix::run_home_manager(&ctx))?;
        runner.execute(Step::Asdf, "asdf", || unix::run_asdf(&ctx))?;
        runner.execute(Step::Pkgin, "pkgin", || unix::run_pkgin(&ctx))?;
        runner.execute(Step::Bun, "bun", || unix::run_bun(&ctx))?;
        runner.execute(Step::BunPackages, "bun-packages", || unix::run_bun_packages(&ctx))?;
        runner.execute(Step::Shell, "zr", || zsh::run_zr(&ctx))?;
        runner.execute(Step::Shell, "antibody", || zsh::run_antibody(&ctx))?;
        runner.execute(Step::Shell, "antidote", || zsh::run_antidote(&ctx))?;
        runner.execute(Step::Shell, "antigen", || zsh::run_antigen(&ctx))?;
        runner.execute(Step::Shell, "zgenom", || zsh::run_zgenom(&ctx))?;
        runner.execute(Step::Shell, "zplug", || zsh::run_zplug(&ctx))?;
        runner.execute(Step::Shell, "zinit", || zsh::run_zinit(&ctx))?;
        runner.execute(Step::Shell, "zi", || zsh::run_zi(&ctx))?;
        runner.execute(Step::Shell, "zim", || zsh::run_zim(&ctx))?;
        runner.execute(Step::Shell, "oh-my-zsh", || zsh::run_oh_my_zsh(&ctx))?;
        runner.execute(Step::Shell, "oh-my-bash", || unix::run_oh_my_bash(&ctx))?;
        runner.execute(Step::Shell, "fisher", || unix::run_fisher(&ctx))?;
        runner.execute(Step::Shell, "bash-it", || unix::run_bashit(&ctx))?;
        runner.execute(Step::Shell, "oh-my-fish", || unix::run_oh_my_fish(&ctx))?;
        runner.execute(Step::Shell, "fish-plug", || unix::run_fish_plug(&ctx))?;
        runner.execute(Step::Shell, "fundle", || unix::run_fundle(&ctx))?;
        runner.execute(Step::Tmux, "tmux", || tmux::run_tpm(&ctx))?;
        runner.execute(Step::Tldr, "TLDR", || unix::run_tldr(&ctx))?;
        runner.execute(Step::Pearl, "pearl", || unix::run_pearl(&ctx))?;
        #[cfg(not(any(target_os = "macos", target_os = "android")))]
        runner.execute(Step::GnomeShellExtensions, "Gnome Shell Extensions", || {
            unix::upgrade_gnome_extensions(&ctx)
        })?;
        runner.execute(Step::Sdkman, "SDKMAN!", || unix::run_sdkman(&ctx))?;
        runner.execute(Step::Rcm, "rcm", || unix::run_rcm(&ctx))?;
        runner.execute(Step::Maza, "maza", || unix::run_maza(&ctx))?;
    }

    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    {
        runner.execute(Step::Atom, "apm", || generic::run_apm(&ctx))?;
    }

    // The following update function should be executed on all OSes.
    runner.execute(Step::Fossil, "fossil", || generic::run_fossil(&ctx))?;
    runner.execute(Step::Rustup, "rustup", || generic::run_rustup(&ctx))?;
    runner.execute(Step::Juliaup, "juliaup", || generic::run_juliaup(&ctx))?;
    runner.execute(Step::Dotnet, ".NET", || generic::run_dotnet_upgrade(&ctx))?;
    runner.execute(Step::Choosenim, "choosenim", || generic::run_choosenim(&ctx))?;
    runner.execute(Step::Cargo, "cargo", || generic::run_cargo_update(&ctx))?;
    runner.execute(Step::Flutter, "Flutter", || generic::run_flutter_upgrade(&ctx))?;
    runner.execute(Step::Go, "go-global-update", || go::run_go_global_update(&ctx))?;
    runner.execute(Step::Go, "gup", || go::run_go_gup(&ctx))?;
    runner.execute(Step::Emacs, "Emacs", || emacs.upgrade(&ctx))?;
    runner.execute(Step::Opam, "opam", || generic::run_opam_update(&ctx))?;
    runner.execute(Step::Vcpkg, "vcpkg", || generic::run_vcpkg_update(&ctx))?;
    runner.execute(Step::Pipx, "pipx", || generic::run_pipx_update(&ctx))?;
    runner.execute(Step::Vscode, "Visual Studio Code extensions", || {
        generic::run_vscode_extensions_upgrade(&ctx)
    })?;
    runner.execute(Step::Conda, "conda", || generic::run_conda_update(&ctx))?;
    runner.execute(Step::Mamba, "mamba", || generic::run_mamba_update(&ctx))?;
    runner.execute(Step::Miktex, "miktex", || generic::run_miktex_packages_update(&ctx))?;
    runner.execute(Step::Pip3, "pip3", || generic::run_pip3_update(&ctx))?;
    runner.execute(Step::PipReview, "pip-review", || generic::run_pip_review_update(&ctx))?;
    runner.execute(Step::PipReviewLocal, "pip-review (local)", || {
        generic::run_pip_review_local_update(&ctx)
    })?;
    runner.execute(Step::Pipupgrade, "pipupgrade", || generic::run_pipupgrade_update(&ctx))?;
    runner.execute(Step::Ghcup, "ghcup", || generic::run_ghcup_update(&ctx))?;
    runner.execute(Step::Stack, "stack", || generic::run_stack_update(&ctx))?;
    runner.execute(Step::Tlmgr, "tlmgr", || generic::run_tlmgr_update(&ctx))?;
    runner.execute(Step::Myrepos, "myrepos", || generic::run_myrepos_update(&ctx))?;
    runner.execute(Step::Chezmoi, "chezmoi", || generic::run_chezmoi_update(&ctx))?;
    runner.execute(Step::Jetpack, "jetpack", || generic::run_jetpack(&ctx))?;
    runner.execute(Step::Vim, "vim", || vim::upgrade_vim(&ctx))?;
    runner.execute(Step::Vim, "Neovim", || vim::upgrade_neovim(&ctx))?;
    runner.execute(Step::Vim, "The Ultimate vimrc", || vim::upgrade_ultimate_vimrc(&ctx))?;
    runner.execute(Step::Vim, "voom", || vim::run_voom(&ctx))?;
    runner.execute(Step::Kakoune, "Kakoune", || kakoune::upgrade_kak_plug(&ctx))?;
    runner.execute(Step::Helix, "helix", || generic::run_helix_grammars(&ctx))?;
    runner.execute(Step::Node, "npm", || node::run_npm_upgrade(&ctx))?;
    runner.execute(Step::Yarn, "yarn", || node::run_yarn_upgrade(&ctx))?;
    runner.execute(Step::Pnpm, "pnpm", || node::run_pnpm_upgrade(&ctx))?;
    runner.execute(Step::Containers, "Containers", || containers::run_containers(&ctx))?;
    runner.execute(Step::Deno, "deno", || node::deno_upgrade(&ctx))?;
    runner.execute(Step::Composer, "composer", || generic::run_composer_update(&ctx))?;
    runner.execute(Step::Krew, "krew", || generic::run_krew_upgrade(&ctx))?;
    runner.execute(Step::Helm, "helm", || generic::run_helm_repo_update(&ctx))?;
    runner.execute(Step::Gem, "gem", || generic::run_gem(&ctx))?;
    runner.execute(Step::RubyGems, "rubygems", || generic::run_rubygems(&ctx))?;
    runner.execute(Step::Julia, "julia", || generic::update_julia_packages(&ctx))?;
    runner.execute(Step::Haxelib, "haxelib", || generic::run_haxelib_update(&ctx))?;
    runner.execute(Step::Sheldon, "sheldon", || generic::run_sheldon(&ctx))?;
    runner.execute(Step::Stew, "stew", || generic::run_stew(&ctx))?;
    runner.execute(Step::Rtcl, "rtcl", || generic::run_rtcl(&ctx))?;
    runner.execute(Step::Bin, "bin", || generic::bin_update(&ctx))?;
    runner.execute(Step::Gcloud, "gcloud", || generic::run_gcloud_components_update(&ctx))?;
    runner.execute(Step::Micro, "micro", || generic::run_micro(&ctx))?;
    runner.execute(Step::Raco, "raco", || generic::run_raco_update(&ctx))?;
    runner.execute(Step::Spicetify, "spicetify", || generic::spicetify_upgrade(&ctx))?;
    runner.execute(Step::GithubCliExtensions, "GitHub CLI Extensions", || {
        generic::run_ghcli_extensions_upgrade(&ctx)
    })?;
    runner.execute(Step::Bob, "Bob", || generic::run_bob(&ctx))?;

    if config.use_predefined_git_repos() {
        if config.should_run(Step::Emacs) {
            if !emacs.is_doom() {
                if let Some(directory) = emacs.directory() {
                    git_repos.insert_if_repo(directory);
                }
            }
            git_repos.insert_if_repo(HOME_DIR.join(".doom.d"));
        }

        if config.should_run(Step::Vim) {
            git_repos.insert_if_repo(HOME_DIR.join(".vim"));
            git_repos.insert_if_repo(HOME_DIR.join(".config/nvim"));
        }

        git_repos.insert_if_repo(HOME_DIR.join(".ideavimrc"));
        git_repos.insert_if_repo(HOME_DIR.join(".intellimacs"));

        if config.should_run(Step::Rcm) {
            git_repos.insert_if_repo(HOME_DIR.join(".dotfiles"));
        }

        #[cfg(unix)]
        {
            git_repos.insert_if_repo(zsh::zshrc());
            if config.should_run(Step::Tmux) {
                git_repos.insert_if_repo(HOME_DIR.join(".tmux"));
            }
            git_repos.insert_if_repo(HOME_DIR.join(".config/fish"));
            git_repos.insert_if_repo(CONFIG_DIR.join("openbox"));
            git_repos.insert_if_repo(CONFIG_DIR.join("bspwm"));
            git_repos.insert_if_repo(CONFIG_DIR.join("i3"));
            git_repos.insert_if_repo(CONFIG_DIR.join("sway"));
        }

        #[cfg(windows)]
        git_repos.insert_if_repo(CACHE_DIR.join("Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState"));

        #[cfg(windows)]
        windows::insert_startup_scripts(&mut git_repos).ok();

        if let Some(profile) = powershell.profile() {
            git_repos.insert_if_repo(profile);
        }
    }

    if config.should_run(Step::GitRepos) {
        if let Some(custom_git_repos) = config.git_repos() {
            for git_repo in custom_git_repos {
                git_repos.glob_insert(git_repo);
            }
        }
        runner.execute(Step::GitRepos, "Git repositories", || {
            git.multi_pull_step(&git_repos, &ctx)
        })?;
    }

    if should_run_powershell {
        runner.execute(Step::Powershell, "Powershell Modules Update", || {
            powershell.update_modules(&ctx)
        })?;
    }

    if let Some(commands) = config.commands() {
        for (name, command) in commands {
            if config.should_run_custom_command(name) {
                runner.execute(Step::CustomCommands, name, || {
                    generic::run_custom_command(name, command, &ctx)
                })?;
            }
        }
    }

    if config.should_run(Step::Vagrant) {
        if let Ok(boxes) = vagrant::collect_boxes(&ctx) {
            for vagrant_box in boxes {
                runner.execute(Step::Vagrant, format!("Vagrant ({})", vagrant_box.smart_name()), || {
                    vagrant::topgrade_vagrant_box(&ctx, &vagrant_box)
                })?;
            }
        }
    }
    runner.execute(Step::Vagrant, "Vagrant boxes", || vagrant::upgrade_vagrant_boxes(&ctx))?;

    if !runner.report().data().is_empty() {
        print_separator("Summary");

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
        print_info("\n(R)eboot\n(S)hell\n(Q)uit");
        loop {
            match get_key() {
                Ok(Key::Char('s')) | Ok(Key::Char('S')) => {
                    run_shell().context("Failed to execute shell")?;
                }
                Ok(Key::Char('r')) | Ok(Key::Char('R')) => {
                    reboot().context("Failed to reboot")?;
                }
                Ok(Key::Char('q')) | Ok(Key::Char('Q')) => (),
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
            format!(
                "Topgrade finished {}",
                if failed { "with errors" } else { "successfully" }
            ),
            Some(Duration::from_secs(10)),
        )
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
                println!("Error: {error:?}");
            }
            exit(1);
        }
    }
}
