#![allow(clippy::cognitive_complexity)]

use std::env;
use std::io;
use std::process::exit;

use anyhow::{anyhow, Result};
use clap::{crate_version, Parser};
use console::Key;
use log::debug;
use log::LevelFilter;
use pretty_env_logger::formatted_timed_builder;

use self::config::{CommandLineArgs, Config, Step};
use self::error::StepFailed;
#[cfg(all(windows, feature = "self-update"))]
use self::error::Upgraded;
use self::steps::{remote::*, *};
use self::terminal::*;

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
mod terminal;
mod utils;

fn run() -> Result<()> {
    ctrlc::set_handler();

    let base_dirs = directories::BaseDirs::new().ok_or_else(|| anyhow!("No base directories"))?;

    let opt = CommandLineArgs::parse();

    for env in opt.env_variables() {
        let mut splitted = env.split('=');
        let var = splitted.next().unwrap();
        let value = splitted.next().unwrap();
        env::set_var(var, value);
    }

    let mut builder = formatted_timed_builder();

    if opt.verbose {
        builder.filter(Some("topgrade"), LevelFilter::Trace);
    }

    builder.init();

    if opt.edit_config() {
        Config::edit(&base_dirs)?;
        return Ok(());
    };

    if opt.show_config_reference() {
        print!("{}", crate::config::EXAMPLE_CONFIG);
        return Ok(());
    }

    let config = Config::load(&base_dirs, opt)?;
    terminal::set_title(config.set_title());
    terminal::display_time(config.display_time());
    terminal::set_desktop_notifications(config.notify_each_step());

    debug!("Version: {}", crate_version!());
    debug!("OS: {}", env!("TARGET"));
    debug!("{:?}", std::env::args());
    debug!("Binary path: {:?}", std::env::current_exe());
    debug!("Self Update: {:?}", cfg!(feature = "self-update"));

    if config.run_in_tmux() && env::var("TOPGRADE_INSIDE_TMUX").is_err() {
        #[cfg(unix)]
        {
            tmux::run_in_tmux(config.tmux_arguments());
        }
    }

    let git = git::Git::new();
    let mut git_repos = git::Repositories::new(&git);

    let sudo = utils::sudo();
    let run_type = executor::RunType::new(config.dry_run());

    let ctx = execution_context::ExecutionContext::new(run_type, &sudo, &git, &config, &base_dirs);

    let mut runner = runner::Runner::new(&ctx);

    #[cfg(feature = "self-update")]
    {
        if !run_type.dry() && env::var("TOPGRADE_NO_SELF_UPGRADE").is_err() {
            let result = self_update::self_update();

            if let Err(e) = &result {
                #[cfg(windows)]
                {
                    if e.downcast_ref::<Upgraded>().is_some() {
                        return result;
                    }
                }
                print_warning(format!("Self update error: {}", e));
            }
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

    let powershell = powershell::Powershell::new();
    let should_run_powershell = powershell.profile().is_some() && config.should_run(Step::Powershell);

    #[cfg(windows)]
    runner.execute(Step::Wsl, "WSL", || windows::run_wsl_topgrade(&ctx))?;

    if let Some(topgrades) = config.remote_topgrades() {
        for remote_topgrade in topgrades.iter().filter(|t| config.should_execute_remote(t)) {
            runner.execute(Step::Remotes, format!("Remote ({})", remote_topgrade), || {
                remote::ssh::ssh_step(&ctx, remote_topgrade)
            })?;
        }
    }

    #[cfg(target_os = "linux")]
    let distribution = linux::Distribution::detect();

    #[cfg(target_os = r#"linux"#)]
    {
        match &distribution {
            Ok(distribution) => {
                runner.execute(Step::System, "System update", || distribution.upgrade(&ctx))?;
            }
            Err(e) => {
                println!("Error detecting current distribution: {}", e);
            }
        }
        runner.execute(Step::ConfigUpdate, "config-update", || linux::run_config_update(&ctx))?;

        runner.execute(Step::BrewFormula, "Brew", || {
            unix::run_brew_formula(&ctx, unix::BrewVariant::Path)
        })?;
    }

    #[cfg(windows)]
    {
        runner.execute(Step::Chocolatey, "Chocolatey", || windows::run_chocolatey(&ctx))?;
        runner.execute(Step::Scoop, "Scoop", || windows::run_scoop(config.cleanup(), run_type))?;
        runner.execute(Step::Winget, "Winget", || windows::run_winget(&ctx))?;
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
    }

    #[cfg(unix)]
    {
        runner.execute(Step::Yadm, "yadm", || unix::run_yadm(&ctx))?;
        runner.execute(Step::Nix, "nix", || unix::run_nix(&ctx))?;
        runner.execute(Step::Guix, "guix", || unix::run_guix(&ctx))?;

        runner.execute(Step::HomeManager, "home-manager", || unix::run_home_manager(run_type))?;
        runner.execute(Step::Asdf, "asdf", || unix::run_asdf(run_type))?;
        runner.execute(Step::Pkgin, "pkgin", || unix::run_pkgin(&ctx))?;
        runner.execute(Step::Bun, "bun", || unix::run_bun(&ctx))?;
    }

    #[cfg(target_os = "dragonfly")]
    runner.execute(Step::Pkg, "DragonFly BSD Packages", || {
        dragonfly::upgrade_packages(sudo.as_ref(), run_type)
    })?;

    #[cfg(target_os = "freebsd")]
    runner.execute(Step::Pkg, "FreeBSD Packages", || {
        freebsd::upgrade_packages(sudo.as_ref(), run_type)
    })?;

    #[cfg(target_os = "openbsd")]
    runner.execute(Step::Pkg, "OpenBSD Packages", || {
        openbsd::upgrade_packages(sudo.as_ref(), run_type)
    })?;

    #[cfg(target_os = "android")]
    runner.execute(Step::Pkg, "Termux Packages", || android::upgrade_packages(&ctx))?;

    let emacs = emacs::Emacs::new(&base_dirs);
    if config.use_predefined_git_repos() {
        if config.should_run(Step::Emacs) {
            if !emacs.is_doom() {
                if let Some(directory) = emacs.directory() {
                    git_repos.insert_if_repo(directory);
                }
            }
            git_repos.insert_if_repo(base_dirs.home_dir().join(".doom.d"));
        }

        if config.should_run(Step::Vim) {
            git_repos.insert_if_repo(base_dirs.home_dir().join(".vim"));
            git_repos.insert_if_repo(base_dirs.home_dir().join(".config/nvim"));
        }

        git_repos.insert_if_repo(base_dirs.home_dir().join(".ideavimrc"));
        git_repos.insert_if_repo(base_dirs.home_dir().join(".intellimacs"));

        if config.should_run(Step::Rcm) {
            git_repos.insert_if_repo(base_dirs.home_dir().join(".dotfiles"));
        }

        #[cfg(unix)]
        {
            git_repos.insert_if_repo(zsh::zshrc(&base_dirs));
            if config.should_run(Step::Tmux) {
                git_repos.insert_if_repo(base_dirs.home_dir().join(".tmux"));
            }
            git_repos.insert_if_repo(base_dirs.home_dir().join(".config/fish"));
            git_repos.insert_if_repo(base_dirs.config_dir().join("openbox"));
            git_repos.insert_if_repo(base_dirs.config_dir().join("bspwm"));
            git_repos.insert_if_repo(base_dirs.config_dir().join("i3"));
            git_repos.insert_if_repo(base_dirs.config_dir().join("sway"));
        }

        #[cfg(windows)]
        git_repos.insert_if_repo(
            base_dirs
                .data_local_dir()
                .join("Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState"),
        );

        #[cfg(windows)]
        windows::insert_startup_scripts(&ctx, &mut git_repos).ok();

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

    #[cfg(unix)]
    {
        runner.execute(Step::Shell, "zr", || zsh::run_zr(&base_dirs, run_type))?;
        runner.execute(Step::Shell, "antibody", || zsh::run_antibody(run_type))?;
        runner.execute(Step::Shell, "antigen", || zsh::run_antigen(&base_dirs, run_type))?;
        runner.execute(Step::Shell, "zgenom", || zsh::run_zgenom(&base_dirs, run_type))?;
        runner.execute(Step::Shell, "zplug", || zsh::run_zplug(&base_dirs, run_type))?;
        runner.execute(Step::Shell, "zinit", || zsh::run_zinit(&base_dirs, run_type))?;
        runner.execute(Step::Shell, "zi", || zsh::run_zi(&base_dirs, run_type))?;
        runner.execute(Step::Shell, "zim", || zsh::run_zim(&base_dirs, run_type))?;
        runner.execute(Step::Shell, "oh-my-zsh", || zsh::run_oh_my_zsh(&ctx))?;
        runner.execute(Step::Shell, "fisher", || unix::run_fisher(&base_dirs, run_type))?;
        runner.execute(Step::Shell, "bash-it", || unix::run_bashit(&ctx))?;
        runner.execute(Step::Shell, "oh-my-fish", || unix::run_oh_my_fish(&ctx))?;
        runner.execute(Step::Shell, "fish-plug", || unix::run_fish_plug(&ctx))?;
        runner.execute(Step::Shell, "fundle", || unix::run_fundle(&ctx))?;
        runner.execute(Step::Tmux, "tmux", || tmux::run_tpm(&base_dirs, run_type))?;
        runner.execute(Step::Tldr, "TLDR", || unix::run_tldr(run_type))?;
        runner.execute(Step::Pearl, "pearl", || unix::run_pearl(run_type))?;
        #[cfg(not(any(target_os = "macos", target_os = "android")))]
        runner.execute(Step::GnomeShellExtensions, "Gnome Shell Extensions", || {
            unix::upgrade_gnome_extensions(&ctx)
        })?;
        runner.execute(Step::Sdkman, "SDKMAN!", || {
            unix::run_sdkman(&base_dirs, config.cleanup(), run_type)
        })?;
        runner.execute(Step::Rcm, "rcm", || unix::run_rcm(&ctx))?;
    }

    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    runner.execute(Step::Atom, "apm", || generic::run_apm(run_type))?;
    runner.execute(Step::Fossil, "fossil", || generic::run_fossil(run_type))?;
    runner.execute(Step::Rustup, "rustup", || generic::run_rustup(&base_dirs, run_type))?;
    runner.execute(Step::Dotnet, ".NET", || generic::run_dotnet_upgrade(&ctx))?;
    runner.execute(Step::Choosenim, "choosenim", || generic::run_choosenim(&ctx))?;
    runner.execute(Step::Cargo, "cargo", || generic::run_cargo_update(&ctx))?;
    runner.execute(Step::Flutter, "Flutter", || generic::run_flutter_upgrade(run_type))?;
    runner.execute(Step::Go, "Go", || generic::run_go(run_type))?;
    runner.execute(Step::Emacs, "Emacs", || emacs.upgrade(&ctx))?;
    runner.execute(Step::Opam, "opam", || generic::run_opam_update(&ctx))?;
    runner.execute(Step::Vcpkg, "vcpkg", || generic::run_vcpkg_update(run_type))?;
    runner.execute(Step::Pipx, "pipx", || generic::run_pipx_update(run_type))?;
    runner.execute(Step::Conda, "conda", || generic::run_conda_update(&ctx))?;
    runner.execute(Step::Pip3, "pip3", || generic::run_pip3_update(run_type))?;
    runner.execute(Step::Ghcup, "ghcup", || generic::run_ghcup_update(run_type))?;
    runner.execute(Step::Stack, "stack", || generic::run_stack_update(run_type))?;
    runner.execute(Step::Tlmgr, "tlmgr", || generic::run_tlmgr_update(&ctx))?;
    runner.execute(Step::Myrepos, "myrepos", || {
        generic::run_myrepos_update(&base_dirs, run_type)
    })?;
    runner.execute(Step::Chezmoi, "chezmoi", || {
        generic::run_chezmoi_update(&base_dirs, run_type)
    })?;
    runner.execute(Step::Jetpack, "jetpack", || generic::run_jetpack(run_type))?;
    runner.execute(Step::Vim, "vim", || vim::upgrade_vim(&base_dirs, &ctx))?;
    runner.execute(Step::Vim, "Neovim", || vim::upgrade_neovim(&base_dirs, &ctx))?;
    runner.execute(Step::Vim, "The Ultimate vimrc", || vim::upgrade_ultimate_vimrc(&ctx))?;
    runner.execute(Step::Vim, "voom", || vim::run_voom(&base_dirs, run_type))?;
    runner.execute(Step::Kakoune, "Kakoune", || kakoune::upgrade_kak_plug(&ctx))?;
    runner.execute(Step::Node, "npm", || node::run_npm_upgrade(&ctx))?;
    runner.execute(Step::Node, "yarn", || node::run_yarn_upgrade(&ctx))?;
    runner.execute(Step::Node, "pnpm", || node::run_pnpm_upgrade(&ctx))?;
    runner.execute(Step::Containers, "Containers", || containers::run_containers(&ctx))?;
    runner.execute(Step::Deno, "deno", || node::deno_upgrade(&ctx))?;
    runner.execute(Step::Composer, "composer", || generic::run_composer_update(&ctx))?;
    runner.execute(Step::Krew, "krew", || generic::run_krew_upgrade(run_type))?;
    runner.execute(Step::Gem, "gem", || generic::run_gem(&base_dirs, run_type))?;
    runner.execute(Step::Julia, "julia", || generic::update_julia_packages(&ctx))?;
    runner.execute(Step::Haxelib, "haxelib", || generic::run_haxelib_update(&ctx))?;
    runner.execute(Step::Sheldon, "sheldon", || generic::run_sheldon(&ctx))?;
    runner.execute(Step::Rtcl, "rtcl", || generic::run_rtcl(&ctx))?;
    runner.execute(Step::Bin, "bin", || generic::bin_update(&ctx))?;
    runner.execute(Step::Gcloud, "gcloud", || {
        generic::run_gcloud_components_update(run_type)
    })?;
    runner.execute(Step::Micro, "micro", || generic::run_micro(run_type))?;
    runner.execute(Step::Raco, "raco", || generic::run_raco_update(run_type))?;
    runner.execute(Step::Spicetify, "spicetify", || generic::spicetify_upgrade(&ctx))?;
    runner.execute(Step::GithubCliExtensions, "GitHub CLI Extensions", || {
        generic::run_ghcli_extensions_upgrade(&ctx)
    })?;

    #[cfg(target_os = "linux")]
    {
        runner.execute(Step::DebGet, "deb-get", || linux::run_deb_get(&ctx))?;
        runner.execute(Step::Toolbx, "toolbx", || toolbx::run_toolbx(&ctx))?;
        runner.execute(Step::Flatpak, "Flatpak", || linux::flatpak_update(&ctx))?;
        runner.execute(Step::Snap, "snap", || linux::run_snap(sudo.as_ref(), run_type))?;
        runner.execute(Step::Pacstall, "pacstall", || linux::run_pacstall(&ctx))?;
        runner.execute(Step::Pacdef, "pacdef", || linux::run_pacdef(&ctx))?;
        runner.execute(Step::Protonup, "protonup", || linux::run_protonup_update(&ctx))?;
        runner.execute(Step::Distrobox, "distrobox", || linux::run_distrobox_update(&ctx))?;
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

    #[cfg(target_os = "linux")]
    {
        runner.execute(Step::System, "pihole", || {
            linux::run_pihole_update(sudo.as_ref(), run_type)
        })?;
        runner.execute(Step::Firmware, "Firmware upgrades", || linux::run_fwupdmgr(&ctx))?;
        runner.execute(Step::Restarts, "Restarts", || {
            linux::run_needrestart(sudo.as_ref(), run_type)
        })?;
    }

    #[cfg(target_os = "macos")]
    {
        runner.execute(Step::Sparkle, "Sparkle", || macos::run_sparkle(&ctx))?;
        runner.execute(Step::Mas, "App Store", || macos::run_mas(run_type))?;
        runner.execute(Step::System, "System upgrade", || macos::upgrade_macos(&ctx))?;
    }

    #[cfg(target_os = "freebsd")]
    runner.execute(Step::System, "FreeBSD Upgrade", || {
        freebsd::upgrade_freebsd(sudo.as_ref(), run_type)
    })?;

    #[cfg(target_os = "openbsd")]
    runner.execute(Step::System, "OpenBSD Upgrade", || {
        openbsd::upgrade_openbsd(sudo.as_ref(), run_type)
    })?;

    #[cfg(windows)]
    runner.execute(Step::System, "Windows update", || windows::windows_update(&ctx))?;

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

        #[cfg(target_os = "freebsd")]
        freebsd::audit_packages(&sudo).ok();

        #[cfg(target_os = "dragonfly")]
        dragonfly::audit_packages(&sudo).ok();
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
                    run_shell();
                }
                Ok(Key::Char('r')) | Ok(Key::Char('R')) => {
                    reboot();
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
        terminal::notify_desktop(
            format!(
                "Topgrade finished {}",
                if failed { "with errors" } else { "successfully" }
            ),
            None,
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
                println!("Error: {}", error);
            }
            exit(1);
        }
    }
}
