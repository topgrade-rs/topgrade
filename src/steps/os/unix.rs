use color_eyre::eyre::eyre;
use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use etcetera::BaseStrategy;
use home;
use ini::Ini;
#[cfg(target_os = "linux")]
use nix::unistd::Uid;
use regex::Regex;
use rust_i18n::t;
use semver::Version;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Component;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;
use std::{env::var, path::Path};
use tracing::{debug, warn};

use crate::command::CommandExt;
use crate::sudo::SudoExecuteOpts;
use crate::XDG_DIRS;
use crate::{output_changed_message, HOME_DIR};

#[cfg(target_os = "linux")]
use super::linux::Distribution;
use crate::error::{SkipStep, StepFailed};
use crate::execution_context::ExecutionContext;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::executor::Executor;
use crate::step::Step;
use crate::terminal::print_separator;
use crate::utils::{require, PathExt};

#[cfg(any(target_os = "linux", target_os = "macos"))]
const INTEL_BREW: &str = "/usr/local/bin/brew";

#[cfg(any(target_os = "linux", target_os = "macos"))]
const ARM_BREW: &str = "/opt/homebrew/bin/brew";

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub enum BrewVariant {
    Path,
    MacIntel,
    MacArm,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl BrewVariant {
    fn binary_name(self) -> &'static str {
        match self {
            BrewVariant::Path => "brew",
            BrewVariant::MacIntel => INTEL_BREW,
            BrewVariant::MacArm => ARM_BREW,
        }
    }

    #[cfg(target_os = "macos")]
    fn is_path(&self) -> bool {
        matches!(self, BrewVariant::Path)
    }

    fn both_both_exist() -> bool {
        Path::new(INTEL_BREW).exists() && Path::new(ARM_BREW).exists()
    }

    pub fn step_title(self) -> &'static str {
        let both_exists = Self::both_both_exist();
        match self {
            BrewVariant::MacArm if both_exists => "Brew (ARM)",
            BrewVariant::MacIntel if both_exists => "Brew (Intel)",
            _ => "Brew",
        }
    }

    /// Execute an "internal" brew command, i.e. one that should always be run
    /// even when dry-running. Basically just a wrapper around [`Command::new`]
    /// that uses `arch` to run using the correct architecture if needed.
    #[cfg(target_os = "macos")]
    fn execute_internal(self) -> Command {
        match self {
            BrewVariant::MacIntel if cfg!(target_arch = "aarch64") => {
                let mut command = Command::new("arch");
                command.arg("-x86_64").arg(self.binary_name());
                command
            }
            BrewVariant::MacArm if cfg!(target_arch = "x86_64") => {
                let mut command = Command::new("arch");
                command.arg("-arm64e").arg(self.binary_name());
                command
            }
            _ => Command::new(self.binary_name()),
        }
    }

    /// Execute a brew command. Uses `arch` to run using the correct
    /// architecture on macOS if needed.
    fn execute(self, ctx: &ExecutionContext) -> Executor {
        match self {
            BrewVariant::MacIntel if cfg!(target_arch = "aarch64") => {
                let mut command = ctx.execute("arch");
                command.arg("-x86_64").arg(self.binary_name());
                command
            }
            BrewVariant::MacArm if cfg!(target_arch = "x86_64") => {
                let mut command = ctx.execute("arch");
                command.arg("-arm64e").arg(self.binary_name());
                command
            }
            _ => ctx.execute(self.binary_name()),
        }
    }

    #[cfg(target_os = "macos")]
    fn is_macos_custom(binary_name: PathBuf) -> bool {
        !(binary_name.as_os_str() == INTEL_BREW || binary_name.as_os_str() == ARM_BREW)
    }
}

pub fn run_fisher(ctx: &ExecutionContext) -> Result<()> {
    let fish = require("fish")?;

    Command::new(&fish)
        .args(["-c", "type -t fisher"])
        .output_checked_utf8()
        .map(|_| ())
        .map_err(|_| SkipStep(t!("`fisher` is not defined in `fish`").to_string()))?;

    Command::new(&fish)
        .args(["-c", "echo \"$__fish_config_dir/fish_plugins\""])
        .output_checked_utf8()
        .and_then(|output| Path::new(&output.stdout.trim()).require().map(|_| ()))
        .map_err(|err| SkipStep(t!("`fish_plugins` path doesn't exist: {err}", err = err).to_string()))?;

    Command::new(&fish)
        .args(["-c", "fish_update_completions"])
        .output_checked_utf8()
        .map(|_| ())
        .map_err(|_| SkipStep(t!("`fish_update_completions` is not available").to_string()))?;

    print_separator("Fisher");

    let version_str = ctx
        .execute(&fish)
        .args(["-c", "fisher --version"])
        .output_checked_utf8()?
        .stdout;
    debug!("Fisher version: {}", version_str);

    if version_str.starts_with("fisher version 3.") {
        // v3 - see https://github.com/topgrade-rs/topgrade/pull/37#issuecomment-1283844506
        ctx.execute(&fish).args(["-c", "fisher"]).status_checked()
    } else {
        // v4
        ctx.execute(&fish).args(["-c", "fisher update"]).status_checked()
    }
}

pub fn run_bashit(ctx: &ExecutionContext) -> Result<()> {
    HOME_DIR.join(".bash_it").require()?;

    print_separator("Bash-it");

    ctx.execute("bash")
        .args(["-lic", &format!("bash-it update {}", ctx.config().bashit_branch())])
        .status_checked()
}

pub fn run_oh_my_bash(ctx: &ExecutionContext) -> Result<()> {
    require("bash")?;
    let oh_my_bash = var("OSH")
        // default to `~/.oh-my-bash`
        .unwrap_or(
            HOME_DIR
                .join(".oh-my-bash")
                .to_str()
                .expect("should be UTF-8 encoded")
                .to_string(),
        )
        .require()?;

    print_separator("oh-my-bash");

    let mut update_script = oh_my_bash;
    update_script.push_str("/tools/upgrade.sh");

    ctx.execute("bash").arg(update_script).status_checked()
}

pub fn run_oh_my_fish(ctx: &ExecutionContext) -> Result<()> {
    let fish = require("fish")?;
    HOME_DIR.join(".local/share/omf/pkg/omf/functions/omf.fish").require()?;

    print_separator("oh-my-fish");

    ctx.execute(fish).args(["-c", "omf update"]).status_checked()
}

pub fn run_pkgin(ctx: &ExecutionContext) -> Result<()> {
    let pkgin = require("pkgin")?;

    print_separator("Pkgin");

    let sudo = ctx.require_sudo()?;

    let mut command = sudo.execute(ctx, &pkgin)?;
    command.arg("update");
    if ctx.config().yes(Step::Pkgin) {
        command.arg("-y");
    }
    command.status_checked()?;

    let mut command = sudo.execute(ctx, &pkgin)?;
    command.arg("upgrade");
    if ctx.config().yes(Step::Pkgin) {
        command.arg("-y");
    }
    command.status_checked()
}

pub fn run_fish_plug(ctx: &ExecutionContext) -> Result<()> {
    let fish = require("fish")?;
    HOME_DIR
        .join(".local/share/fish/plug/kidonng/fish-plug/functions/plug.fish")
        .require()?;

    print_separator("fish-plug");

    ctx.execute(fish).args(["-c", "plug update"]).status_checked()
}

/// Upgrades `fundle` and `fundle` plugins.
///
/// `fundle` is a package manager for the Fish shell.
///
/// See: <https://github.com/danhper/fundle>
pub fn run_fundle(ctx: &ExecutionContext) -> Result<()> {
    let fish = require("fish")?;
    HOME_DIR.join(".config/fish/fundle").require()?;

    print_separator("fundle");

    ctx.execute(fish)
        .args(["-c", "fundle self-update && fundle update"])
        .status_checked()
}

#[cfg(not(any(target_os = "android", target_os = "macos")))]
pub fn upgrade_gnome_extensions(ctx: &ExecutionContext) -> Result<()> {
    let gdbus = require("gdbus")?;
    crate::utils::require_option(
        var("XDG_CURRENT_DESKTOP").ok().filter(|p| p.contains("GNOME")),
        t!("Desktop does not appear to be GNOME").to_string(),
    )?;
    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.DBus",
            "--object-path",
            "/org/freedesktop/DBus",
            "--method",
            "org.freedesktop.DBus.ListActivatableNames",
        ])
        .output_checked_utf8()?;

    debug!("Checking for GNOME extensions: {}", output);
    if !output.stdout.contains("org.gnome.Shell.Extensions") {
        return Err(SkipStep(t!("GNOME shell extensions are unregistered in DBus").to_string()).into());
    }

    print_separator(t!("GNOME Shell extensions"));

    ctx.execute(gdbus)
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell.Extensions",
            "--object-path",
            "/org/gnome/Shell/Extensions",
            "--method",
            "org.gnome.Shell.Extensions.CheckForUpdates",
        ])
        .status_checked()
}

#[cfg(target_os = "linux")]
pub fn brew_linux_sudo_uid() -> Option<u32> {
    let linuxbrew_directory = "/home/linuxbrew/.linuxbrew";
    if let Ok(metadata) = std::fs::metadata(linuxbrew_directory) {
        let owner_id = metadata.uid();
        let current_id = Uid::effective();
        // print debug these two values
        debug!("linuxbrew_directory owner_id: {}, current_id: {}", owner_id, current_id);
        return if owner_id == current_id.as_raw() {
            None // no need for sudo if linuxbrew is owned by the current user
        } else {
            Some(owner_id) // otherwise use sudo to run brew as the owner
        };
    }
    None
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn run_brew_formula(ctx: &ExecutionContext, variant: BrewVariant) -> Result<()> {
    #[allow(unused_variables)]
    let binary_name = require(variant.binary_name())?;

    #[cfg(target_os = "macos")]
    {
        if variant.is_path() && !BrewVariant::is_macos_custom(binary_name) {
            return Err(SkipStep(t!("Not a custom brew for macOS").to_string()).into());
        }
    }

    #[cfg(target_os = "linux")]
    {
        let sudo_uid = brew_linux_sudo_uid();
        // if brew is owned by another user, execute "sudo -Hu <uid> brew update"
        if let Some(user_id) = sudo_uid {
            let uid = nix::unistd::Uid::from_raw(user_id);
            let user = nix::unistd::User::from_uid(uid)
                .expect("failed to call getpwuid()")
                .expect("this user should exist");

            let sudo_as_user = t!("sudo as user '{user}'", user = user.name);
            print_separator(format!("{} ({})", variant.step_title(), sudo_as_user));

            let sudo = ctx.require_sudo()?;
            sudo.execute_opts(ctx, &binary_name, SudoExecuteOpts::new().set_home().user(&user.name))?
                .current_dir("/tmp") // brew needs a writable current directory
                .arg("update")
                .status_checked()?;
            return Ok(());
        }
    }
    print_separator(variant.step_title());

    variant.execute(ctx).arg("update").status_checked()?;

    let mut command = variant.execute(ctx);
    command.args(["upgrade", "--formula"]);

    if ctx.config().brew_fetch_head() {
        command.arg("--fetch-HEAD");
    }

    command.status_checked()?;

    if ctx.config().cleanup() {
        variant.execute(ctx).arg("cleanup").status_checked()?;
    }

    if ctx.config().brew_autoremove() {
        variant.execute(ctx).arg("autoremove").status_checked()?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn run_brew_cask(ctx: &ExecutionContext, variant: BrewVariant) -> Result<()> {
    let binary_name = require(variant.binary_name())?;
    if variant.is_path() && !BrewVariant::is_macos_custom(binary_name) {
        return Err(SkipStep(t!("Not a custom brew for macOS").to_string()).into());
    }
    print_separator(format!("{} - Cask", variant.step_title()));

    let cask_upgrade_exists = variant
        .execute_internal()
        .args(["--repository", "buo/cask-upgrade"])
        .output_checked_utf8()
        .map(|p| Path::new(p.stdout.trim()).exists())?;

    let mut brew_args = vec![];

    if cask_upgrade_exists {
        brew_args.extend(["cu", "-y"]);
        if ctx.config().brew_cask_greedy() {
            brew_args.push("-a");
        }
    } else {
        brew_args.extend(["upgrade", "--cask"]);
        if ctx.config().brew_cask_greedy() {
            brew_args.push("--greedy");
        }
        if ctx.config().brew_greedy_latest() {
            brew_args.push("--greedy-latest");
        }
        if ctx.config().brew_greedy_auto_updates() {
            brew_args.push("--greedy-auto-updates");
        }
    }

    variant.execute(ctx).args(&brew_args).status_checked()?;

    if ctx.config().cleanup() {
        variant.execute(ctx).arg("cleanup").status_checked()?;
    }

    Ok(())
}

pub fn run_guix(ctx: &ExecutionContext) -> Result<()> {
    let guix = require("guix")?;

    print_separator("Guix");

    ctx.execute(&guix).arg("pull").status_checked()?;
    ctx.execute(&guix).args(["package", "-u"]).status_checked()?;

    Ok(())
}

struct NixVersion {
    version_string: String,
}

impl NixVersion {
    fn new(ctx: &ExecutionContext, nix: &Path) -> Result<Self> {
        let version_output = ctx.execute(nix).arg("--version").output_checked_utf8()?;

        debug!(
            output=%version_output,
            "`nix --version` output"
        );

        let version_string = version_output
            .stdout
            .lines()
            .next()
            .ok_or_else(|| eyre!("`nix --version` output is empty"))?
            .to_string();

        if version_string.is_empty() {
            return Err(eyre!("`nix --version` output was empty"));
        }

        Ok(Self { version_string })
    }

    fn version(&self) -> Result<Version> {
        static NIX_VERSION_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^nix \([^)]*\) ([0-9.]+)").expect("Nix version regex always compiles"));

        let captures = NIX_VERSION_REGEX
            .captures(&self.version_string)
            .ok_or_else(|| eyre!(output_changed_message!("nix --version", "regex did not match")))?;
        let raw_version = &captures[1];

        debug!("Raw Nix version: {raw_version}");

        // Nix 2.29.0 outputs "2.29" instead of "2.29.0", so we need to add that if necessary.
        let corrected_raw_version = if raw_version.chars().filter(|&c| c == '.').count() == 1 {
            &format!("{raw_version}.0")
        } else {
            raw_version
        };

        debug!("Corrected raw Nix version: {corrected_raw_version}");

        let version = Version::parse(corrected_raw_version)
            .wrap_err_with(|| output_changed_message!("nix --version", "Invalid version"))?;

        debug!("Nix version: {:?}", version);

        Ok(version)
    }

    fn is_lix(&self) -> bool {
        let is_lix = self.version_string.contains("Lix");
        debug!(?is_lix);
        is_lix
    }

    fn is_determinate_nix(&self) -> bool {
        let is_determinate_nix = self.version_string.contains("Determinate Nix");
        debug!(?is_determinate_nix);
        is_determinate_nix
    }
}

pub fn run_nix(ctx: &ExecutionContext) -> Result<()> {
    let nix = require("nix")?;
    let nix_channel = require("nix-channel")?;
    let nix_env = require("nix-env")?;
    // TODO: Is None possible here?
    let profile_path = match home::home_dir() {
        Some(home) => XDG_DIRS
            .state_dir()
            .map(|d| d.join("nix/profile"))
            .filter(|p| p.exists())
            .unwrap_or(Path::new(&home).join(".nix-profile")),
        None => Path::new("/nix/var/nix/profiles/per-user/default").into(),
    };
    debug!("nix profile: {:?}", profile_path);
    let manifest_json_path = profile_path.join("manifest.json");

    print_separator("Nix");

    #[cfg(target_os = "macos")]
    {
        if require("darwin-rebuild").is_ok() {
            return Err(
                SkipStep(t!("Nix-darwin on macOS must be upgraded via darwin-rebuild switch").to_string()).into(),
            );
        }
    }

    ctx.execute(nix_channel).arg("--update").status_checked()?;

    let nix_version = NixVersion::new(ctx, &nix)?;

    // Nix since 2.21.0 uses `--all --impure` rather than `.*` to upgrade all packages.
    // Lix is based on Nix 2.18, so it doesn't!
    let packages = if nix_version.version()? >= Version::new(2, 21, 0) && !nix_version.is_lix() {
        vec!["--all", "--impure"]
    } else {
        vec![".*"]
    };

    if Path::new(&manifest_json_path).exists() {
        ctx.execute(nix)
            .args(nix_args())
            .arg("profile")
            .arg("upgrade")
            .args(&packages)
            .arg("--verbose")
            .status_checked()
    } else {
        let mut command = ctx.execute(nix_env);
        command.arg("--upgrade");
        if let Some(args) = ctx.config().nix_env_arguments() {
            command.args(args.split_whitespace());
        };
        command.status_checked()
    }
}

pub fn run_nix_self_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let nix = require("nix")?;

    // Should we attempt to upgrade Nix with `nix upgrade-nix`?
    #[allow(unused_mut)]
    let mut should_self_upgrade = cfg!(target_os = "macos");

    #[cfg(target_os = "linux")]
    {
        // We can't use `nix upgrade-nix` on NixOS.
        if let Ok(Distribution::NixOS) = Distribution::detect() {
            should_self_upgrade = false;
        }
    }

    if !should_self_upgrade {
        return Err(SkipStep(t!("`nix upgrade-nix` can only be used on macOS or non-NixOS Linux").to_string()).into());
    }

    if nix_profile_dir(&nix)?.is_none() {
        return Err(
            SkipStep(t!("`nix upgrade-nix` cannot be run when Nix is installed in a profile").to_string()).into(),
        );
    }

    print_separator(t!("Nix (self-upgrade)"));

    let nix_version = NixVersion::new(ctx, &nix)?;

    if nix_version.is_determinate_nix() {
        let nixd = require("determinate-nixd");
        let nixd = match nixd {
            Err(_) => {
                println!("Found Determinate Nix, but could not find determinate-nixd");
                return Err(StepFailed.into());
            }
            Ok(nixd) => nixd,
        };

        let sudo = ctx.require_sudo()?;
        return sudo
            .execute_opts(ctx, nixd, SudoExecuteOpts::new().login_shell())?
            .arg("upgrade")
            .status_checked();
    }

    let multi_user = fs::metadata(&nix)?.uid() == 0;
    debug!("Multi user nix: {}", multi_user);

    let nix_args = nix_args();
    if multi_user {
        let sudo = ctx.require_sudo()?;
        sudo.execute_opts(ctx, &nix, SudoExecuteOpts::new().login_shell())?
            .args(nix_args)
            .arg("upgrade-nix")
            .status_checked()
    } else {
        ctx.execute(&nix).args(nix_args).arg("upgrade-nix").status_checked()
    }
}

/// If we try to `nix upgrade-nix` but Nix is installed with `nix profile`, we'll get a `does not
/// appear to be part of a Nix profile` error.
///
/// We duplicate some of the `nix` logic here to avoid this.
/// See: <https://github.com/NixOS/nix/blob/f0180487a0e4c0091b46cb1469c44144f5400240/src/nix/upgrade-nix.cc#L102-L139>
///
/// See: <https://github.com/NixOS/nix/issues/5473>
fn nix_profile_dir(nix: &Path) -> Result<Option<PathBuf>> {
    // NOTE: `nix` uses the location of the `nix-env` binary for this but we're using the `nix`
    // binary; should be the same.
    let nix_bin_dir = nix.parent();
    if nix_bin_dir.and_then(|p| p.file_name()) != Some(OsStr::new("bin")) {
        debug!("Nix is not installed in a `bin` directory: {nix_bin_dir:?}");
        return Ok(None);
    }

    let nix_dir = nix_bin_dir
        .and_then(|bin_dir| bin_dir.parent())
        .ok_or_else(|| eyre!("Unable to find Nix install directory from Nix binary {nix:?}"))?;

    debug!("Found Nix in {nix_dir:?}");

    let mut profile_dir = nix_dir.to_path_buf();
    while profile_dir.is_symlink() {
        profile_dir = profile_dir
            .parent()
            .ok_or_else(|| eyre!("Path has no parent: {profile_dir:?}"))?
            .join(
                profile_dir
                    .read_link()
                    .wrap_err_with(|| format!("Failed to read symlink {profile_dir:?}"))?,
            );

        // NOTE: `nix` uses a hand-rolled canonicalize function, Rust just uses `realpath`.
        if profile_dir
            .canonicalize()
            .wrap_err_with(|| format!("Failed to canonicalize {profile_dir:?}"))?
            .components()
            .any(|component| component == Component::Normal(OsStr::new("profiles")))
        {
            break;
        }
    }

    debug!("Found Nix profile {profile_dir:?}");
    let user_env = profile_dir
        .canonicalize()
        .wrap_err_with(|| format!("Failed to canonicalize {profile_dir:?}"))?;

    Ok(
        if user_env
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("user-environment"))
        {
            Some(profile_dir)
        } else {
            None
        },
    )
}

/// Returns a directory from an environment variable, if and only if it is a directory which
/// contains a flake.nix
fn flake_dir(var: &'static str) -> Option<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .take_if(|x| std::fs::exists(x.join("flake.nix")).is_ok_and(|x| x))
}

/// Update NixOS and home-manager through a flake using `nh`
///
/// See: https://github.com/viperML/nh
pub fn run_nix_helper(ctx: &ExecutionContext) -> Result<()> {
    require("nix")?;
    let nix_helper = require("nh")?;

    let fallback_flake_path = flake_dir("NH_FLAKE");
    let darwin_flake_path = flake_dir("NH_DARWIN_FLAKE");
    let home_flake_path = flake_dir("NH_HOME_FLAKE");
    let nixos_flake_path = flake_dir("NH_OS_FLAKE");

    let all_flake_paths: Vec<_> = [
        fallback_flake_path.as_ref(),
        darwin_flake_path.as_ref(),
        home_flake_path.as_ref(),
        nixos_flake_path.as_ref(),
    ]
    .into_iter()
    .flatten()
    .collect();

    // if none of the paths exist AND contain a `flake.nix`, skip
    if all_flake_paths.is_empty() {
        if flake_dir("FLAKE").is_some() {
            warn!(
                "{}",
                t!("You have a flake inside of $FLAKE. This is deprecated for nh.")
            );
        }
        return Err(SkipStep(t!("nh cannot find any configured flakes").into()).into());
    }

    let nh_switch = |ty: &'static str| -> Result<()> {
        print_separator(format!("nh {ty}"));

        let mut cmd = ctx.execute(&nix_helper);
        cmd.arg(ty);
        cmd.arg("switch");
        cmd.arg("-u");

        if !ctx.config().yes(Step::NixHelper) {
            cmd.arg("--ask");
        }
        cmd.status_checked()?;
        Ok(())
    };

    // We assume that if the user has set these variables, we can throw an error if nh cannot find
    // a flake there. So we do not anymore perform an eval check to find out whether we should skip
    // or not.
    #[cfg(target_os = "macos")]
    if darwin_flake_path.is_some() || fallback_flake_path.is_some() {
        nh_switch("darwin")?;
    }

    if home_flake_path.is_some() || fallback_flake_path.is_some() {
        nh_switch("home")?;
    }

    #[cfg(target_os = "linux")]
    if matches!(Distribution::detect(), Ok(Distribution::NixOS))
        && (nixos_flake_path.is_some() || fallback_flake_path.is_some())
    {
        nh_switch("os")?;
    }

    Ok(())
}

fn nix_args() -> [&'static str; 2] {
    ["--extra-experimental-features", "nix-command"]
}

pub fn run_yadm(ctx: &ExecutionContext) -> Result<()> {
    let yadm = require("yadm")?;

    print_separator("yadm");

    ctx.execute(yadm).arg("pull").status_checked()
}

pub fn run_asdf(ctx: &ExecutionContext) -> Result<()> {
    let asdf = require("asdf")?;

    print_separator("asdf");

    // asdf (>= 0.15.0) won't support the self-update command
    //
    // https://github.com/topgrade-rs/topgrade/issues/1007
    let version_output = Command::new(&asdf).arg("version").output_checked_utf8()?;
    // Example output
    //
    // ```
    // $ asdf version
    // v0.15.0-31e8c93
    //
    // ```
    // ```
    // $ asdf version
    // v0.16.7
    // ```
    // ```
    // $ asdf version
    // 0.18.0 (revision unknown)
    // ```
    let version_stdout = version_output.stdout.trim();
    // trim the starting 'v'
    let mut remaining = version_stdout.trim_start_matches('v');
    // remove the hash or revision part if present
    if let Some(idx) = remaining.find(['-', ' ']) {
        remaining = &remaining[..idx];
    }
    let version =
        Version::parse(remaining).wrap_err_with(|| output_changed_message!("asdf version", "invalid version"))?;
    if version < Version::new(0, 15, 0) {
        ctx.execute(&asdf).arg("update").status_checked_with_codes(&[42])?;
    }

    ctx.execute(&asdf).args(["plugin", "update", "--all"]).status_checked()
}

pub fn run_mise(ctx: &ExecutionContext) -> Result<()> {
    let mise = require("mise")?;

    print_separator("mise");

    ctx.execute(&mise).args(["plugins", "update"]).status_checked()?;

    ctx.execute(&mise).arg("upgrade").status_checked()
}

pub fn run_home_manager(ctx: &ExecutionContext) -> Result<()> {
    let home_manager = require("home-manager")?;

    print_separator("home-manager");

    let mut cmd = ctx.execute(home_manager);
    cmd.arg("switch");

    if let Some(extra_args) = ctx.config().home_manager() {
        cmd.args(extra_args);
    }

    cmd.status_checked()
}

pub fn run_pearl(ctx: &ExecutionContext) -> Result<()> {
    let pearl = require("pearl")?;
    print_separator("pearl");

    ctx.execute(pearl).arg("update").status_checked()
}

pub fn run_pyenv(ctx: &ExecutionContext) -> Result<()> {
    let pyenv = require("pyenv")?;
    print_separator("pyenv");

    let pyenv_dir = var("PYENV_ROOT").map_or_else(|_| HOME_DIR.join(".pyenv"), PathBuf::from);

    if !pyenv_dir.exists() {
        return Err(SkipStep(t!("Pyenv is installed, but $PYENV_ROOT is not set correctly").to_string()).into());
    }

    if !pyenv_dir.join(".git").exists() {
        return Err(SkipStep(t!("pyenv is not a git repository").to_string()).into());
    }

    if !pyenv_dir.join("plugins").join("pyenv-update").exists() {
        return Err(SkipStep(t!("pyenv-update plugin is not installed").to_string()).into());
    }

    ctx.execute(pyenv).arg("update").status_checked()
}

pub fn run_sdkman(ctx: &ExecutionContext) -> Result<()> {
    let bash = require("bash")?;

    let sdkman_init_path = var("SDKMAN_DIR")
        .map_or_else(|_| HOME_DIR.join(".sdkman"), PathBuf::from)
        .join("bin")
        .join("sdkman-init.sh")
        .require()
        .map(|p| format!("{}", &p.display()))?;

    print_separator("SDKMAN!");

    let sdkman_config_path = var("SDKMAN_DIR")
        .map_or_else(|_| HOME_DIR.join(".sdkman"), PathBuf::from)
        .join("etc")
        .join("config")
        .require()?;

    let sdkman_config = Ini::load_from_file(sdkman_config_path)?;
    let selfupdate_enabled = sdkman_config
        .general_section()
        .get("sdkman_selfupdate_feature")
        .unwrap_or("false");

    if selfupdate_enabled == "true" {
        let cmd_selfupdate = format!("source {} && sdk selfupdate", &sdkman_init_path);
        ctx.execute(&bash)
            .args(["-c", cmd_selfupdate.as_str()])
            .status_checked()?;
    }

    let cmd_update = format!("source {} && sdk update", &sdkman_init_path);
    ctx.execute(&bash).args(["-c", cmd_update.as_str()]).status_checked()?;

    let cmd_upgrade = format!("source {} && sdk upgrade", &sdkman_init_path);
    ctx.execute(&bash).args(["-c", cmd_upgrade.as_str()]).status_checked()?;

    if ctx.config().cleanup() {
        let cmd_flush_archives = format!("source {} && sdk flush archives", &sdkman_init_path);
        ctx.execute(&bash)
            .args(["-c", cmd_flush_archives.as_str()])
            .status_checked()?;

        let cmd_flush_temp = format!("source {} && sdk flush temp", &sdkman_init_path);
        ctx.execute(&bash)
            .args(["-c", cmd_flush_temp.as_str()])
            .status_checked()?;
    }

    Ok(())
}

pub fn run_bun_packages(ctx: &ExecutionContext) -> Result<()> {
    let bun = require("bun")?;

    print_separator(t!("Bun Packages"));

    let mut package_json: PathBuf = var("BUN_INSTALL").map_or_else(|_| HOME_DIR.join(".bun"), PathBuf::from);
    package_json.push("install/global/package.json");

    if !package_json.exists() {
        println!("{}", t!("No global packages installed"));
        return Ok(());
    }

    ctx.execute(bun).args(["-g", "update"]).status_checked()
}

/// Update dotfiles with `rcm(7)`.
///
/// See: <https://github.com/thoughtbot/rcm>
pub fn run_rcm(ctx: &ExecutionContext) -> Result<()> {
    let rcup = require("rcup")?;

    print_separator("rcm");
    ctx.execute(rcup).arg("-v").status_checked()
}

pub fn run_maza(ctx: &ExecutionContext) -> Result<()> {
    let maza = require("maza")?;

    print_separator("maza");
    ctx.execute(maza).arg("update").status_checked()
}

pub fn run_hyprpm(ctx: &ExecutionContext) -> Result<()> {
    let hyprpm = require("hyprpm")?;

    print_separator("hyprpm");

    ctx.execute(hyprpm).arg("update").status_checked()
}

pub fn run_atuin(ctx: &ExecutionContext) -> Result<()> {
    let atuin = require("atuin-update")?;

    print_separator("atuin");

    ctx.execute(atuin).status_checked()
}

pub fn reboot(ctx: &ExecutionContext) -> Result<()> {
    match ctx.sudo() {
        Some(sudo) => sudo.execute(ctx, "reboot")?.status_checked(),
        None => ctx.execute("reboot").status_checked(),
    }
}
