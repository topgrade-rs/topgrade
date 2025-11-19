use crate::execution_context::ExecutionContext;
use crate::runner::Runner;
use clap::ValueEnum;
use color_eyre::Result;
#[cfg(target_os = "linux")]
use rust_i18n::t;
use serde::Deserialize;
use strum::{EnumCount, EnumIter, EnumString, VariantNames};

#[cfg(feature = "self-update")]
use crate::self_update;
use crate::steps::remote::vagrant;
#[allow(clippy::wildcard_imports)]
use crate::steps::*;
use crate::utils::hostname;

#[derive(ValueEnum, EnumString, VariantNames, Debug, Clone, PartialEq, Eq, Deserialize, EnumIter, Copy, EnumCount)]
#[clap(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Step {
    AM,
    AndroidStudio,
    AppMan,
    Aqua,
    Asdf,
    Atom,
    Atuin,
    Audit,
    AutoCpufreq,
    Bin,
    Bob,
    BrewCask,
    BrewFormula,
    Bun,
    BunPackages,
    Cargo,
    Certbot,
    Chezmoi,
    Chocolatey,
    Choosenim,
    CinnamonSpices,
    ClamAvDb,
    Composer,
    Conda,
    ConfigUpdate,
    Containers,
    CustomCommands,
    DebGet,
    Deno,
    Distrobox,
    DkpPacman,
    Dotnet,
    Elan,
    Emacs,
    Falconf,
    Firmware,
    Flatpak,
    Flutter,
    Fossil,
    Gcloud,
    Gem,
    Ghcup,
    GitRepos,
    GithubCliExtensions,
    GnomeShellExtensions,
    Go,
    Guix,
    Haxelib,
    Helix,
    Helm,
    HomeManager,
    Hyprpm,
    // These names are miscapitalized on purpose, so the CLI name is
    //  `jetbrains_pycharm` instead of `jet_brains_py_charm`.
    JetbrainsAqua,
    JetbrainsClion,
    JetbrainsDatagrip,
    JetbrainsDataspell,
    JetbrainsGateway,
    JetbrainsGoland,
    JetbrainsIdea,
    JetbrainsMps,
    JetbrainsPhpstorm,
    JetbrainsPycharm,
    JetbrainsRider,
    JetbrainsRubymine,
    JetbrainsRustrover,
    JetbrainsToolbox,
    JetbrainsWebstorm,
    Jetpack,
    Julia,
    Juliaup,
    Kakoune,
    Krew,
    Lensfun,
    Lure,
    Macports,
    Mamba,
    Mandb,
    Mas,
    Maza,
    Micro,
    MicrosoftStore,
    Miktex,
    Mise,
    Myrepos,
    Nix,
    NixHelper,
    Node,
    Opam,
    Pacdef,
    Pacstall,
    Pearl,
    Pip3,
    PipReview,
    PipReviewLocal,
    Pipupgrade,
    Pipx,
    Pipxu,
    Pixi,
    Pkg,
    Pkgfile,
    Pkgin,
    PlatformioCore,
    Pnpm,
    Poetry,
    Powershell,
    Protonup,
    Pyenv,
    Raco,
    Rcm,
    Remotes,
    Restarts,
    Rtcl,
    RubyGems,
    Rustup,
    Rye,
    Scoop,
    Sdkman,
    SelfUpdate,
    Sheldon,
    Shell,
    Snap,
    Sparkle,
    Spicetify,
    Stack,
    Stew,
    System,
    Tldr,
    Tlmgr,
    Tmux,
    Toolbx,
    Typst,
    Uv,
    Vagrant,
    Vcpkg,
    Vim,
    VoltaPackages,
    Vscode,
    VscodeInsiders,
    Vscodium,
    VscodiumInsiders,
    Waydroid,
    Winget,
    Wsl,
    WslUpdate,
    Xcodes,
    Yadm,
    Yarn,
    Yazi,
    Zigup,
    Zvm,
}

impl Step {
    #[allow(clippy::too_many_lines)]
    pub fn run(&self, runner: &mut Runner, ctx: &ExecutionContext) -> Result<()> {
        use Step::*;

        match *self {
            AM =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "am", || linux::run_am(ctx))?
            }
            AndroidStudio => runner.execute(*self, "Android Studio Plugins", || generic::run_android_studio(ctx))?,
            AppMan =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "appman", || linux::run_appman(ctx))?
            }
            Aqua => runner.execute(*self, "aqua", || generic::run_aqua(ctx))?,
            Asdf =>
            {
                #[cfg(unix)]
                runner.execute(*self, "asdf", || unix::run_asdf(ctx))?
            }
            Atom =>
            {
                #[cfg(not(any(
                    target_os = "freebsd",
                    target_os = "openbsd",
                    target_os = "netbsd",
                    target_os = "dragonfly"
                )))]
                runner.execute(*self, "apm", || generic::run_apm(ctx))?
            }
            Atuin =>
            {
                #[cfg(unix)]
                runner.execute(*self, "atuin", || unix::run_atuin(ctx))?
            }
            Audit => {
                #[cfg(target_os = "dragonfly")]
                runner.execute(*self, "DragonFly Audit", || dragonfly::audit_packages(ctx))?;
                #[cfg(target_os = "freebsd")]
                runner.execute(*self, "FreeBSD Audit", || freebsd::audit_packages(ctx))?
            }
            AutoCpufreq =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "auto-cpufreq", || linux::run_auto_cpufreq(ctx))?
            }
            Bin => runner.execute(*self, "bin", || generic::bin_update(ctx))?,
            Bob => runner.execute(*self, "Bob", || generic::run_bob(ctx))?,
            BrewCask => {
                #[cfg(target_os = "macos")]
                runner.execute(*self, "Brew Cask", || unix::run_brew_cask(ctx, unix::BrewVariant::Path))?;
                #[cfg(target_os = "macos")]
                runner.execute(*self, "Brew Cask (Intel)", || {
                    unix::run_brew_cask(ctx, unix::BrewVariant::MacIntel)
                })?;
                #[cfg(target_os = "macos")]
                runner.execute(*self, "Brew Cask (ARM)", || {
                    unix::run_brew_cask(ctx, unix::BrewVariant::MacArm)
                })?
            }
            BrewFormula => {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "Brew", || unix::run_brew_formula(ctx, unix::BrewVariant::Path))?;
                #[cfg(target_os = "macos")]
                runner.execute(*self, "Brew (ARM)", || {
                    unix::run_brew_formula(ctx, unix::BrewVariant::MacArm)
                })?;
                #[cfg(target_os = "macos")]
                runner.execute(*self, "Brew (Intel)", || {
                    unix::run_brew_formula(ctx, unix::BrewVariant::MacIntel)
                })?
            }
            Bun => runner.execute(*self, "bun", || generic::run_bun(ctx))?,
            BunPackages =>
            {
                #[cfg(unix)]
                runner.execute(*self, "bun-packages", || unix::run_bun_packages(ctx))?
            }
            Cargo => runner.execute(*self, "cargo", || generic::run_cargo_update(ctx))?,
            Certbot => runner.execute(*self, "Certbot", || generic::run_certbot(ctx))?,
            Chezmoi => runner.execute(*self, "chezmoi", || generic::run_chezmoi_update(ctx))?,
            Chocolatey =>
            {
                #[cfg(windows)]
                runner.execute(*self, "Chocolatey", || windows::run_chocolatey(ctx))?
            }
            Choosenim => runner.execute(*self, "choosenim", || generic::run_choosenim(ctx))?,
            CinnamonSpices =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "Cinnamon spices", || linux::run_cinnamon_spices_updater(ctx))?
            }
            ClamAvDb => runner.execute(*self, "ClamAV Databases", || generic::run_freshclam(ctx))?,
            Composer => runner.execute(*self, "composer", || generic::run_composer_update(ctx))?,
            Conda => runner.execute(*self, "conda", || generic::run_conda_update(ctx))?,
            ConfigUpdate =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "config-update", || linux::run_config_update(ctx))?
            }
            Containers => runner.execute(*self, "Containers", || containers::run_containers(ctx))?,
            CustomCommands => {
                if let Some(commands) = ctx.config().commands() {
                    for (name, command) in commands
                        .iter()
                        .filter(|(n, _)| ctx.config().should_run_custom_command(n))
                    {
                        runner.execute(*self, name.clone(), || generic::run_custom_command(name, command, ctx))?;
                    }
                }
            }
            DebGet =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "deb-get", || linux::run_deb_get(ctx))?
            }
            Deno => runner.execute(*self, "deno", || node::deno_upgrade(ctx))?,
            Distrobox =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "distrobox", || linux::run_distrobox_update(ctx))?
            }
            DkpPacman =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "dkp-pacman", || linux::run_dkp_pacman_update(ctx))?
            }
            Dotnet => runner.execute(*self, ".NET", || generic::run_dotnet_upgrade(ctx))?,
            Elan => runner.execute(*self, "elan", || generic::run_elan(ctx))?,
            Emacs => runner.execute(*self, "Emacs", || emacs::Emacs::new().upgrade(ctx))?,
            Falconf => runner.execute(*self, "falconf sync", || generic::run_falconf(ctx))?,
            Firmware =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "Firmware", || linux::run_fwupdmgr(ctx))?
            }
            Flatpak =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "Flatpak", || linux::run_flatpak(ctx))?
            }
            Flutter => runner.execute(*self, "Flutter", || generic::run_flutter_upgrade(ctx))?,
            Fossil => runner.execute(*self, "fossil", || generic::run_fossil(ctx))?,
            Gcloud => runner.execute(*self, "gcloud", || generic::run_gcloud_components_update(ctx))?,
            Gem => runner.execute(*self, "gem", || generic::run_gem(ctx))?,
            Ghcup => runner.execute(*self, "ghcup", || generic::run_ghcup_update(ctx))?,
            GitRepos => runner.execute(*self, "Git Repositories", || git::run_git_pull(ctx))?,
            GithubCliExtensions => runner.execute(*self, "GitHub CLI Extensions", || {
                generic::run_ghcli_extensions_upgrade(ctx)
            })?,
            GnomeShellExtensions =>
            {
                #[cfg(all(unix, not(any(target_os = "macos", target_os = "android"))))]
                runner.execute(*self, "Gnome Shell Extensions", || unix::upgrade_gnome_extensions(ctx))?
            }
            Go => {
                runner.execute(*self, "go-global-update", || go::run_go_global_update(ctx))?;
                runner.execute(*self, "gup", || go::run_go_gup(ctx))?
            }
            Guix =>
            {
                #[cfg(unix)]
                runner.execute(*self, "guix", || unix::run_guix(ctx))?
            }
            Haxelib => runner.execute(*self, "haxelib", || generic::run_haxelib_update(ctx))?,
            Helix => runner.execute(*self, "helix", || generic::run_helix_grammars(ctx))?,
            Helm => runner.execute(*self, "helm", || generic::run_helm_repo_update(ctx))?,
            HomeManager =>
            {
                #[cfg(unix)]
                runner.execute(*self, "home-manager", || unix::run_home_manager(ctx))?
            }
            Hyprpm =>
            {
                #[cfg(unix)]
                runner.execute(*self, "hyprpm", || unix::run_hyprpm(ctx))?
            }
            JetbrainsAqua => runner.execute(*self, "JetBrains Aqua Plugins", || generic::run_jetbrains_aqua(ctx))?,
            JetbrainsClion => runner.execute(*self, "JetBrains CL", || generic::run_jetbrains_clion(ctx))?,
            JetbrainsDatagrip => {
                runner.execute(*self, "JetBrains DataGrip", || generic::run_jetbrains_datagrip(ctx))?
            }
            JetbrainsDataspell => runner.execute(*self, "JetBrains DataSpell Plugins", || {
                generic::run_jetbrains_dataspell(ctx)
            })?,
            JetbrainsGateway => runner.execute(*self, "JetBrains Gateway Plugins", || {
                generic::run_jetbrains_gateway(ctx)
            })?,
            JetbrainsGoland => {
                runner.execute(*self, "JetBrains GoLand Plugins", || generic::run_jetbrains_goland(ctx))?
            }
            JetbrainsIdea => runner.execute(*self, "JetBrains IntelliJ IDEA Plugins", || {
                generic::run_jetbrains_idea(ctx)
            })?,
            JetbrainsMps => runner.execute(*self, "JetBrains MPS Plugins", || generic::run_jetbrains_mps(ctx))?,
            JetbrainsPhpstorm => runner.execute(*self, "JetBrains PhpStorm Plugins", || {
                generic::run_jetbrains_phpstorm(ctx)
            })?,
            JetbrainsPycharm => runner.execute(*self, "JetBrains PyCharm Plugins", || {
                generic::run_jetbrains_pycharm(ctx)
            })?,
            JetbrainsRider => runner.execute(*self, "JetBrains Rider Plugins", || generic::run_jetbrains_rider(ctx))?,
            JetbrainsRubymine => runner.execute(*self, "JetBrains RubyMine Plugins", || {
                generic::run_jetbrains_rubymine(ctx)
            })?,
            JetbrainsRustrover => runner.execute(*self, "JetBrains RustRover Plugins", || {
                generic::run_jetbrains_rustrover(ctx)
            })?,
            JetbrainsToolbox => runner.execute(*self, "JetBrains Toolbox", || generic::run_jetbrains_toolbox(ctx))?,
            JetbrainsWebstorm => runner.execute(*self, "JetBrains WebStorm Plugins", || {
                generic::run_jetbrains_webstorm(ctx)
            })?,
            Jetpack => runner.execute(*self, "jetpack", || generic::run_jetpack(ctx))?,
            Julia => runner.execute(*self, "julia", || generic::update_julia_packages(ctx))?,
            Juliaup => runner.execute(*self, "juliaup", || generic::run_juliaup(ctx))?,
            Kakoune => runner.execute(*self, "Kakoune", || kakoune::upgrade_kak_plug(ctx))?,
            Krew => runner.execute(*self, "krew", || generic::run_krew_upgrade(ctx))?,
            Lensfun => runner.execute(*self, "Lensfun's database update", || {
                generic::run_lensfun_update_data(ctx)
            })?,
            Lure =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "LURE", || linux::run_lure_update(ctx))?
            }
            Macports =>
            {
                #[cfg(target_os = "macos")]
                runner.execute(*self, "MacPorts", || macos::run_macports(ctx))?
            }
            Mamba => runner.execute(*self, "mamba", || generic::run_mamba_update(ctx))?,
            Mandb =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "Manual Entries", || linux::run_mandb(ctx))?
            }
            Mas =>
            {
                #[cfg(target_os = "macos")]
                runner.execute(*self, "App Store", || macos::run_mas(ctx))?
            }
            Maza =>
            {
                #[cfg(unix)]
                runner.execute(*self, "maza", || unix::run_maza(ctx))?
            }
            Micro => runner.execute(*self, "micro", || generic::run_micro(ctx))?,
            MicrosoftStore =>
            {
                #[cfg(windows)]
                runner.execute(*self, "Microsoft Store", || windows::microsoft_store(ctx))?
            }
            Miktex => runner.execute(*self, "miktex", || generic::run_miktex_packages_update(ctx))?,
            Mise =>
            {
                #[cfg(unix)]
                runner.execute(*self, "mise", || unix::run_mise(ctx))?
            }
            Myrepos => runner.execute(*self, "myrepos", || generic::run_myrepos_update(ctx))?,
            Nix => {
                #[cfg(unix)]
                runner.execute(*self, "nix", || unix::run_nix(ctx))?;
                #[cfg(unix)]
                runner.execute(*self, "nix upgrade-nix", || unix::run_nix_self_upgrade(ctx))?
            }
            NixHelper =>
            {
                #[cfg(unix)]
                runner.execute(*self, "nh", || unix::run_nix_helper(ctx))?
            }
            Node => runner.execute(*self, "npm", || node::run_npm_upgrade(ctx))?,
            Opam => runner.execute(*self, "opam", || generic::run_opam_update(ctx))?,
            Pacdef =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "pacdef", || linux::run_pacdef(ctx))?
            }
            Pacstall =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "pacstall", || linux::run_pacstall(ctx))?
            }
            Pearl =>
            {
                #[cfg(unix)]
                runner.execute(*self, "pearl", || unix::run_pearl(ctx))?
            }
            Pip3 => runner.execute(*self, "pip3", || generic::run_pip3_update(ctx))?,
            PipReview => runner.execute(*self, "pip-review", || generic::run_pip_review_update(ctx))?,
            PipReviewLocal => runner.execute(*self, "pip-review (local)", || {
                generic::run_pip_review_local_update(ctx)
            })?,
            Pipupgrade => runner.execute(*self, "pipupgrade", || generic::run_pipupgrade_update(ctx))?,
            Pipx => runner.execute(*self, "pipx", || generic::run_pipx_update(ctx))?,
            Pipxu => runner.execute(*self, "pipxu", || generic::run_pipxu_update(ctx))?,
            Pixi => runner.execute(*self, "pixi", || generic::run_pixi_update(ctx))?,
            Pkg => {
                #[cfg(target_os = "dragonfly")]
                runner.execute(*self, "Dragonfly BSD Packages", || dragonfly::upgrade_packages(ctx))?;
                #[cfg(target_os = "freebsd")]
                runner.execute(*self, "FreeBSD Packages", || freebsd::upgrade_packages(ctx))?;
                #[cfg(target_os = "openbsd")]
                runner.execute(*self, "OpenBSD Packages", || openbsd::upgrade_packages(ctx))?;
                #[cfg(target_os = "android")]
                runner.execute(*self, "Termux Packages", || android::upgrade_packages(ctx))?
            }
            Pkgfile =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "pkgfile", || linux::run_pkgfile(ctx))?
            }
            Pkgin =>
            {
                #[cfg(unix)]
                runner.execute(*self, "pkgin", || unix::run_pkgin(ctx))?
            }
            PlatformioCore => runner.execute(*self, "PlatformIO Core", || generic::run_platform_io(ctx))?,
            Pnpm => runner.execute(*self, "pnpm", || node::run_pnpm_upgrade(ctx))?,
            Poetry => runner.execute(*self, "Poetry", || generic::run_poetry(ctx))?,
            Powershell => runner.execute(*self, "Powershell Modules Update", || generic::run_powershell(ctx))?,
            Protonup =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "protonup", || linux::run_protonup_update(ctx))?
            }
            Pyenv =>
            {
                #[cfg(unix)]
                runner.execute(*self, "pyenv", || unix::run_pyenv(ctx))?
            }
            Raco => runner.execute(*self, "raco", || generic::run_raco_update(ctx))?,
            Rcm =>
            {
                #[cfg(unix)]
                runner.execute(*self, "rcm", || unix::run_rcm(ctx))?
            }
            Remotes => {
                if let Some(topgrades) = ctx.config().remote_topgrades() {
                    for remote_topgrade in topgrades
                        .iter()
                        .filter(|t| ctx.config().should_execute_remote(hostname(), t))
                    {
                        runner.execute(*self, format!("Remote ({remote_topgrade})"), || {
                            crate::ssh::ssh_step(ctx, remote_topgrade)
                        })?;
                    }
                }
            }
            Restarts =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "Restarts", || linux::run_needrestart(ctx))?
            }
            Rtcl => runner.execute(*self, "rtcl", || generic::run_rtcl(ctx))?,
            RubyGems => runner.execute(*self, "rubygems", || generic::run_rubygems(ctx))?,
            Rustup => runner.execute(*self, "rustup", || generic::run_rustup(ctx))?,
            Rye => runner.execute(*self, "rye", || generic::run_rye(ctx))?,
            Scoop =>
            {
                #[cfg(windows)]
                runner.execute(*self, "Scoop", || windows::run_scoop(ctx))?
            }
            Sdkman =>
            {
                #[cfg(unix)]
                runner.execute(*self, "SDKMAN!", || unix::run_sdkman(ctx))?
            }
            SelfUpdate => {
                // Self-Update step, this will execute only if:
                // 1. the `self-update` feature is enabled
                // 2. it is not disabled from configuration (env var/CLI opt/file)
                #[cfg(feature = "self-update")]
                {
                    if std::env::var("TOPGRADE_NO_SELF_UPGRADE").is_err() && !ctx.config().no_self_update() {
                        runner.execute(*self, "Self Update", || self_update::self_update(ctx))?;
                    }
                }
            }
            Sheldon => runner.execute(*self, "sheldon", || generic::run_sheldon(ctx))?,
            Shell => {
                #[cfg(unix)]
                {
                    runner.execute(*self, "zr", || zsh::run_zr(ctx))?;
                    runner.execute(*self, "antibody", || zsh::run_antibody(ctx))?;
                    runner.execute(*self, "antidote", || zsh::run_antidote(ctx))?;
                    runner.execute(*self, "antigen", || zsh::run_antigen(ctx))?;
                    runner.execute(*self, "zgenom", || zsh::run_zgenom(ctx))?;
                    runner.execute(*self, "zplug", || zsh::run_zplug(ctx))?;
                    runner.execute(*self, "zinit", || zsh::run_zinit(ctx))?;
                    runner.execute(*self, "zi", || zsh::run_zi(ctx))?;
                    runner.execute(*self, "zim", || zsh::run_zim(ctx))?;
                    runner.execute(*self, "oh-my-zsh", || zsh::run_oh_my_zsh(ctx))?;
                    runner.execute(*self, "oh-my-bash", || unix::run_oh_my_bash(ctx))?;
                    runner.execute(*self, "fisher", || unix::run_fisher(ctx))?;
                    runner.execute(*self, "bash-it", || unix::run_bashit(ctx))?;
                    runner.execute(*self, "oh-my-fish", || unix::run_oh_my_fish(ctx))?;
                    runner.execute(*self, "fish-plug", || unix::run_fish_plug(ctx))?;
                    runner.execute(*self, "fundle", || unix::run_fundle(ctx))?
                }
            }
            Snap =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "snap", || linux::run_snap(ctx))?
            }
            Sparkle =>
            {
                #[cfg(target_os = "macos")]
                runner.execute(*self, "Sparkle", || macos::run_sparkle(ctx))?
            }
            Spicetify => runner.execute(*self, "spicetify", || generic::spicetify_upgrade(ctx))?,
            Stack => runner.execute(*self, "stack", || generic::run_stack_update(ctx))?,
            Stew => runner.execute(*self, "stew", || generic::run_stew(ctx))?,
            System => {
                #[cfg(target_os = "linux")]
                {
                    // NOTE: Due to breaking `nu` updates, `packer.nu` needs to be updated before `nu` get updated
                    // by other package managers.
                    runner.execute(Shell, "packer.nu", || linux::run_packer_nu(ctx))?;

                    match ctx.distribution() {
                        Ok(distribution) => {
                            runner.execute(*self, "System update", || distribution.upgrade(ctx))?;
                        }
                        Err(e) => {
                            println!("{}", t!("Error detecting current distribution: {error}", error = e));
                        }
                    }
                    runner.execute(*self, "pihole", || linux::run_pihole_update(ctx))?;
                }
                #[cfg(windows)]
                runner.execute(*self, "Windows update", || windows::windows_update(ctx))?;
                #[cfg(target_os = "macos")]
                runner.execute(*self, "System update", || macos::upgrade_macos(ctx))?;
                #[cfg(target_os = "freebsd")]
                runner.execute(*self, "FreeBSD Upgrade", || freebsd::upgrade_freebsd(ctx))?;
                #[cfg(target_os = "openbsd")]
                runner.execute(*self, "OpenBSD Upgrade", || openbsd::upgrade_openbsd(ctx))?
            }
            Tldr => runner.execute(*self, "TLDR", || generic::run_tldr(ctx))?,
            Tlmgr => runner.execute(*self, "tlmgr", || generic::run_tlmgr_update(ctx))?,
            Tmux =>
            {
                #[cfg(unix)]
                runner.execute(*self, "tmux", || tmux::run_tpm(ctx))?
            }
            Toolbx =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "toolbx", || toolbx::run_toolbx(ctx))?
            }
            Typst => runner.execute(*self, "Typst", || generic::run_typst(ctx))?,
            Uv => runner.execute(*self, "uv", || generic::run_uv(ctx))?,
            Vagrant => {
                if ctx.config().should_run(Vagrant) {
                    if let Ok(boxes) = vagrant::collect_boxes(ctx) {
                        for vagrant_box in boxes {
                            runner.execute(*self, format!("Vagrant ({})", vagrant_box.smart_name()), || {
                                vagrant::topgrade_vagrant_box(ctx, &vagrant_box)
                            })?;
                        }
                    }
                }
                runner.execute(*self, "Vagrant boxes", || vagrant::upgrade_vagrant_boxes(ctx))?;
            }
            Vcpkg => runner.execute(*self, "vcpkg", || generic::run_vcpkg_update(ctx))?,
            Vim => {
                runner.execute(*self, "vim", || vim::upgrade_vim(ctx))?;
                runner.execute(*self, "Neovim", || vim::upgrade_neovim(ctx))?;
                runner.execute(*self, "The Ultimate vimrc", || vim::upgrade_ultimate_vimrc(ctx))?;
                runner.execute(*self, "voom", || vim::run_voom(ctx))?
            }
            VoltaPackages => runner.execute(*self, "volta packages", || node::run_volta_packages_upgrade(ctx))?,
            Vscode => runner.execute(*self, "Visual Studio Code extensions", || {
                generic::run_vscode_extensions_update(ctx)
            })?,
            VscodeInsiders => runner.execute(*self, "Visual Studio Code Insiders extensions", || {
                generic::run_vscode_insiders_extensions_update(ctx)
            })?,
            Vscodium => runner.execute(*self, "VSCodium extensions", || {
                generic::run_vscodium_extensions_update(ctx)
            })?,
            VscodiumInsiders => runner.execute(*self, "VSCodium Insiders extensions", || {
                generic::run_vscodium_insiders_extensions_update(ctx)
            })?,
            Waydroid =>
            {
                #[cfg(target_os = "linux")]
                runner.execute(*self, "Waydroid", || linux::run_waydroid(ctx))?
            }
            Winget =>
            {
                #[cfg(windows)]
                runner.execute(*self, "Winget", || windows::run_winget(ctx))?
            }
            Wsl =>
            {
                #[cfg(windows)]
                runner.execute(*self, "WSL", || windows::run_wsl_topgrade(ctx))?
            }
            WslUpdate =>
            {
                #[cfg(windows)]
                runner.execute(*self, "Update WSL", || windows::update_wsl(ctx))?
            }
            Xcodes =>
            {
                #[cfg(target_os = "macos")]
                runner.execute(*self, "Xcodes", || macos::update_xcodes(ctx))?
            }
            Yadm =>
            {
                #[cfg(unix)]
                runner.execute(*self, "yadm", || unix::run_yadm(ctx))?
            }
            Yarn => runner.execute(*self, "yarn", || node::run_yarn_upgrade(ctx))?,
            Yazi => runner.execute(*self, "Yazi packages", || generic::run_yazi(ctx))?,
            Zigup => runner.execute(*self, "zigup", || generic::run_zigup(ctx))?,
            Zvm => runner.execute(*self, "ZVM", || generic::run_zvm(ctx))?,
        }

        Ok(())
    }
}

#[allow(clippy::too_many_lines)]
pub(crate) fn default_steps() -> Vec<Step> {
    // For now, SelfRenamer and SelfUpdate isn't included as they're ran before the other non-steps (pre-commands, sudo, etc)

    use Step::*;
    // Could probably have a smaller starting capacity, but this at least ensures only 2 allocations:
    // initial and shrink
    let mut steps = Vec::with_capacity(Step::COUNT);

    // Not combined with other generic steps to preserve the order as it was in main.rs originally,
    // but this can be changed in the future.
    steps.push(Remotes);

    #[cfg(windows)]
    steps.extend_from_slice(&[Wsl, WslUpdate, Chocolatey, Scoop, Winget, System, MicrosoftStore]);

    #[cfg(target_os = "macos")]
    steps.extend_from_slice(&[BrewFormula, BrewCask, Macports, Xcodes, Sparkle, Mas, System]);

    #[cfg(target_os = "dragonfly")]
    steps.extend_from_slice(&[Pkg, Audit]);

    #[cfg(target_os = "freebsd")]
    steps.extend_from_slice(&[Pkg, System, Audit]);

    #[cfg(target_os = "openbsd")]
    steps.extend_from_slice(&[Pkg, System]);

    #[cfg(target_os = "android")]
    steps.push(Pkg);

    #[cfg(target_os = "linux")]
    steps.extend_from_slice(&[
        System,
        ConfigUpdate,
        AM,
        AppMan,
        DebGet,
        Toolbx,
        Snap,
        Pacstall,
        Pacdef,
        Protonup,
        Distrobox,
        DkpPacman,
        Firmware,
        Restarts,
        Flatpak,
        BrewFormula,
        Lure,
        Waydroid,
        AutoCpufreq,
        CinnamonSpices,
        Mandb,
        Pkgfile,
    ]);

    #[cfg(unix)]
    steps.extend_from_slice(&[
        Yadm,
        Nix,
        NixHelper,
        Guix,
        HomeManager,
        Asdf,
        Mise,
        Pkgin,
        BunPackages,
        Shell,
        Tmux,
        Pearl,
        #[cfg(not(any(target_os = "macos", target_os = "android")))]
        GnomeShellExtensions,
        Pyenv,
        Sdkman,
        Rcm,
        Maza,
        Hyprpm,
        Atuin,
    ]);

    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    steps.push(Atom);

    // The following update function should be executed on all OSes.
    steps.extend_from_slice(&[
        Fossil,
        Elan,
        Rye,
        Rustup,
        Juliaup,
        Dotnet,
        Choosenim,
        Cargo,
        Flutter,
        Go,
        Emacs,
        Opam,
        Vcpkg,
        Pipx,
        Pipxu,
        Vscode,
        VscodeInsiders,
        Vscodium,
        VscodiumInsiders,
        Conda,
        Mamba,
        Pixi,
        Miktex,
        Pip3,
        PipReview,
        PipReviewLocal,
        Pipupgrade,
        Ghcup,
        Stack,
        Tldr,
        Tlmgr,
        Myrepos,
        Chezmoi,
        Jetpack,
        Vim,
        Kakoune,
        Helix,
        Node,
        Yarn,
        Pnpm,
        VoltaPackages,
        Containers,
        Deno,
        Composer,
        Krew,
        Helm,
        Gem,
        RubyGems,
        Julia,
        Haxelib,
        Sheldon,
        Stew,
        Rtcl,
        Bin,
        Gcloud,
        Micro,
        Raco,
        Spicetify,
        GithubCliExtensions,
        Bob,
        Certbot,
        GitRepos,
        ClamAvDb,
        PlatformioCore,
        Lensfun,
        Poetry,
        Uv,
        Zvm,
        Aqua,
        Bun,
        Zigup,
        JetbrainsToolbox,
        AndroidStudio,
        JetbrainsAqua,
        JetbrainsClion,
        JetbrainsDatagrip,
        JetbrainsDataspell,
        // JetBrains dotCover has no CLI
        // JetBrains dotMemory has no CLI
        // JetBrains dotPeek has no CLI
        // JetBrains dotTrace has no CLI
        // JetBrains Fleet has a different CLI without a `fleet update` command.
        JetbrainsGateway,
        JetbrainsGoland,
        JetbrainsIdea,
        JetbrainsMps,
        JetbrainsPhpstorm,
        JetbrainsPycharm,
        // JetBrains ReSharper has no CLI (it's a VSCode extension)
        // JetBrains ReSharper C++ has no CLI (it's a VSCode extension)
        JetbrainsRider,
        JetbrainsRubymine,
        JetbrainsRustrover,
        // JetBrains Space Desktop does not have a CLI
        JetbrainsWebstorm,
        Yazi,
        Falconf,
        Powershell,
        CustomCommands,
        Vagrant,
        Typst,
    ]);

    steps.shrink_to_fit();

    steps
}
