use crate::error::{SkipStep, TopgradeError};
use crate::execution_context::ExecutionContext;
use crate::executor::{CommandExt, Executor, ExecutorExitStatus, RunType};
use crate::terminal::print_separator;
#[cfg(not(target_os = "macos"))]
use crate::utils::require_option;
use crate::utils::{require, PathExt};
use crate::Step;
use anyhow::Result;
use directories::BaseDirs;
use home;
use ini::Ini;
use log::debug;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::Command;
use std::{env, path::Path};

const INTEL_BREW: &str = "/usr/local/bin/brew";
const ARM_BREW: &str = "/opt/homebrew/bin/brew";

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum BrewVariant {
    Path,
    MacIntel,
    MacArm,
}

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

    fn execute(self, run_type: RunType) -> Executor {
        match self {
            BrewVariant::MacIntel if cfg!(target_arch = "aarch64") => {
                let mut command = run_type.execute("arch");
                command.arg("-x86_64").arg(self.binary_name());
                command
            }
            BrewVariant::MacArm if cfg!(target_arch = "x86_64") => {
                let mut command = run_type.execute("arch");
                command.arg("-arm64e").arg(self.binary_name());
                command
            }
            _ => run_type.execute(self.binary_name()),
        }
    }

    #[cfg(target_os = "macos")]
    fn is_macos_custom(binary_name: PathBuf) -> bool {
        !(binary_name.as_os_str() == INTEL_BREW || binary_name.as_os_str() == ARM_BREW)
    }
}

pub fn run_fisher(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let fish = require("fish")?;

    if env::var("fisher_path").is_err() {
        base_dirs
            .home_dir()
            .join(".config/fish/functions/fisher.fish")
            .require()?;
    }

    print_separator("Fisher");

    let version_str = run_type
        .execute(&fish)
        .args(["-c", "fisher --version"])
        .check_output()?;
    debug!("Fisher version: {}", version_str);

    if version_str.starts_with("fisher version 3.") {
        // v3 - see https://github.com/topgrade-rs/topgrade/pull/37#issuecomment-1283844506
        run_type.execute(&fish).args(["-c", "fisher"]).check_run()
    } else {
        // v4
        run_type.execute(&fish).args(["-c", "fisher update"]).check_run()
    }
}

pub fn run_bashit(ctx: &ExecutionContext) -> Result<()> {
    ctx.base_dirs().home_dir().join(".bash_it").require()?;

    print_separator("Bash-it");

    ctx.run_type()
        .execute("bash")
        .args(["-lic", &format!("bash-it update {}", ctx.config().bashit_branch())])
        .check_run()
}

pub fn run_oh_my_fish(ctx: &ExecutionContext) -> Result<()> {
    let fish = require("fish")?;
    ctx.base_dirs()
        .home_dir()
        .join(".local/share/omf/pkg/omf/functions/omf.fish")
        .require()?;

    print_separator("oh-my-fish");

    ctx.run_type().execute(fish).args(["-c", "omf update"]).check_run()
}

pub fn run_pkgin(ctx: &ExecutionContext) -> Result<()> {
    let pkgin = require("pkgin")?;

    let mut command = ctx.run_type().execute(ctx.sudo().as_ref().unwrap());
    command.arg(&pkgin).arg("update");
    if ctx.config().yes(Step::Pkgin) {
        command.arg("-y");
    }
    command.check_run()?;

    let mut command = ctx.run_type().execute(ctx.sudo().as_ref().unwrap());
    command.arg(&pkgin).arg("upgrade");
    if ctx.config().yes(Step::Pkgin) {
        command.arg("-y");
    }
    command.check_run()
}

pub fn run_fish_plug(ctx: &ExecutionContext) -> Result<()> {
    let fish = require("fish")?;
    ctx.base_dirs()
        .home_dir()
        .join(".local/share/fish/plug/kidonng/fish-plug/functions/plug.fish")
        .require()?;

    print_separator("fish-plug");

    ctx.run_type().execute(fish).args(["-c", "plug update"]).check_run()
}

/// Upgrades `fundle` and `fundle` plugins.
///
/// `fundle` is a package manager for the Fish shell.
///
/// See: <https://github.com/danhper/fundle>
pub fn run_fundle(ctx: &ExecutionContext) -> Result<()> {
    let fish = require("fish")?;
    ctx.base_dirs().home_dir().join(".config/fish/fundle").require()?;

    print_separator("fundle");

    ctx.run_type()
        .execute(fish)
        .args(["-c", "fundle self-update && fundle update"])
        .check_run()
}

#[cfg(not(any(target_os = "android", target_os = "macos")))]
pub fn upgrade_gnome_extensions(ctx: &ExecutionContext) -> Result<()> {
    let gdbus = require("gdbus")?;
    require_option(
        env::var("XDG_CURRENT_DESKTOP").ok().filter(|p| p.contains("GNOME")),
        "Desktop doest not appear to be gnome".to_string(),
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
        .check_output()?;

    debug!("Checking for gnome extensions: {}", output);
    if !output.contains("org.gnome.Shell.Extensions") {
        return Err(SkipStep(String::from("Gnome shell extensions are unregistered in DBus")).into());
    }

    print_separator("Gnome Shell extensions");

    ctx.run_type()
        .execute(gdbus)
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
        .check_run()
}

pub fn run_brew_formula(ctx: &ExecutionContext, variant: BrewVariant) -> Result<()> {
    #[allow(unused_variables)]
    let binary_name = require(variant.binary_name())?;

    #[cfg(target_os = "macos")]
    {
        if variant.is_path() && !BrewVariant::is_macos_custom(binary_name) {
            return Err(SkipStep("Not a custom brew for macOS".to_string()).into());
        }
    }

    print_separator(variant.step_title());
    let run_type = ctx.run_type();

    variant.execute(run_type).arg("update").check_run()?;
    variant
        .execute(run_type)
        .args(["upgrade", "--ignore-pinned", "--formula"])
        .check_run()?;

    if ctx.config().cleanup() {
        variant.execute(run_type).arg("cleanup").check_run()?;
    }

    if ctx.config().brew_autoremove() {
        variant.execute(run_type).arg("autoremove").check_run()?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn run_brew_cask(ctx: &ExecutionContext, variant: BrewVariant) -> Result<()> {
    let binary_name = require(variant.binary_name())?;
    if variant.is_path() && !BrewVariant::is_macos_custom(binary_name) {
        return Err(SkipStep("Not a custom brew for macOS".to_string()).into());
    }
    print_separator(format!("{} - Cask", variant.step_title()));
    let run_type = ctx.run_type();

    let cask_upgrade_exists = variant
        .execute(RunType::Wet)
        .args(["--repository", "buo/cask-upgrade"])
        .check_output()
        .map(|p| Path::new(p.trim()).exists())?;

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
    }

    variant.execute(run_type).args(&brew_args).check_run()?;

    if ctx.config().cleanup() {
        variant.execute(run_type).arg("cleanup").check_run()?;
    }

    Ok(())
}

pub fn run_guix(ctx: &ExecutionContext) -> Result<()> {
    let guix = require("guix")?;

    let run_type = ctx.run_type();

    let output = Command::new(&guix).arg("pull").check_output();
    debug!("guix pull output: {:?}", output);
    let should_upgrade = output.is_ok();
    debug!("Can Upgrade Guix: {:?}", should_upgrade);

    print_separator("Guix");

    if should_upgrade {
        return run_type.execute(&guix).args(["package", "-u"]).check_run();
    }
    Err(SkipStep(String::from("Guix Pull Failed, Skipping")).into())
}

pub fn run_nix(ctx: &ExecutionContext) -> Result<()> {
    let nix = require("nix")?;
    let nix_channel = require("nix-channel")?;
    let nix_env = require("nix-env")?;
    let profile_path = match home::home_dir() {
        Some(home) => Path::new(&home).join(".nix-profile"),
        None => Path::new("/nix/var/nix/profiles/per-user/default").into(),
    };
    debug!("nix profile: {:?}", profile_path);
    let manifest_json_path = profile_path.join("manifest.json");

    let output = Command::new(&nix_env).args(["--query", "nix"]).check_output();
    debug!("nix-env output: {:?}", output);
    let should_self_upgrade = output.is_ok();

    print_separator("Nix");

    let multi_user = fs::metadata(&nix)?.uid() == 0;
    debug!("Multi user nix: {}", multi_user);

    #[cfg(target_os = "linux")]
    {
        use super::linux::Distribution;

        if let Ok(Distribution::NixOS) = Distribution::detect() {
            return Err(SkipStep(String::from("Nix on NixOS must be upgraded via nixos-rebuild switch")).into());
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(..) = require("darwin-rebuild") {
            return Err(SkipStep(String::from(
                "Nix-darwin on macOS must be upgraded via darwin-rebuild switch",
            ))
            .into());
        }
    }

    let run_type = ctx.run_type();

    if should_self_upgrade {
        if multi_user {
            ctx.execute_elevated(&nix, true)?.arg("upgrade-nix").check_run()?;
        } else {
            run_type.execute(&nix).arg("upgrade-nix").check_run()?;
        }
    }

    run_type.execute(nix_channel).arg("--update").check_run()?;

    if std::path::Path::new(&manifest_json_path).exists() {
        run_type
            .execute(&nix)
            .arg("profile")
            .arg("upgrade")
            .arg(".*")
            .check_run()
    } else {
        run_type.execute(&nix_env).arg("--upgrade").check_run()
    }
}

pub fn run_yadm(ctx: &ExecutionContext) -> Result<()> {
    let yadm = require("yadm")?;

    print_separator("yadm");

    ctx.run_type().execute(yadm).arg("pull").check_run()
}

pub fn run_asdf(run_type: RunType) -> Result<()> {
    let asdf = require("asdf")?;

    print_separator("asdf");
    let exit_status = run_type.execute(&asdf).arg("update").spawn()?.wait()?;

    if let ExecutorExitStatus::Wet(e) = exit_status {
        if !(e.success() || e.code().map(|c| c == 42).unwrap_or(false)) {
            return Err(TopgradeError::ProcessFailed(e).into());
        }
    }
    run_type.execute(&asdf).args(["plugin", "update", "--all"]).check_run()
}

pub fn run_home_manager(run_type: RunType) -> Result<()> {
    let home_manager = require("home-manager")?;

    print_separator("home-manager");
    run_type.execute(home_manager).arg("switch").check_run()
}

pub fn run_tldr(run_type: RunType) -> Result<()> {
    let tldr = require("tldr")?;

    print_separator("TLDR");
    run_type.execute(tldr).arg("--update").check_run()
}

pub fn run_pearl(run_type: RunType) -> Result<()> {
    let pearl = require("pearl")?;
    print_separator("pearl");

    run_type.execute(pearl).arg("update").check_run()
}

pub fn run_sdkman(base_dirs: &BaseDirs, cleanup: bool, run_type: RunType) -> Result<()> {
    let bash = require("bash")?;

    let sdkman_init_path = env::var("SDKMAN_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base_dirs.home_dir().join(".sdkman"))
        .join("bin")
        .join("sdkman-init.sh")
        .require()
        .map(|p| format!("{}", &p.display()))?;

    print_separator("SDKMAN!");

    let sdkman_config_path = env::var("SDKMAN_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base_dirs.home_dir().join(".sdkman"))
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
        run_type
            .execute(&bash)
            .args(["-c", cmd_selfupdate.as_str()])
            .check_run()?;
    }

    let cmd_update = format!("source {} && sdk update", &sdkman_init_path);
    run_type.execute(&bash).args(["-c", cmd_update.as_str()]).check_run()?;

    let cmd_upgrade = format!("source {} && sdk upgrade", &sdkman_init_path);
    run_type.execute(&bash).args(["-c", cmd_upgrade.as_str()]).check_run()?;

    if cleanup {
        let cmd_flush_archives = format!("source {} && sdk flush archives", &sdkman_init_path);
        run_type
            .execute(&bash)
            .args(["-c", cmd_flush_archives.as_str()])
            .check_run()?;

        let cmd_flush_temp = format!("source {} && sdk flush temp", &sdkman_init_path);
        run_type
            .execute(&bash)
            .args(["-c", cmd_flush_temp.as_str()])
            .check_run()?;
    }

    Ok(())
}

pub fn run_bun(ctx: &ExecutionContext) -> Result<()> {
    let bun = require("bun")?;

    print_separator("Bun");

    ctx.run_type().execute(bun).arg("upgrade").check_run()
}

/// Update dotfiles with `rcm(7)`.
///
/// See: <https://github.com/thoughtbot/rcm>
pub fn run_rcm(ctx: &ExecutionContext) -> Result<()> {
    let rcup = require("rcup")?;

    print_separator("rcm");
    ctx.run_type().execute(rcup).arg("-v").check_run()
}

pub fn reboot() {
    print!("Rebooting...");
    Command::new("sudo").arg("reboot").spawn().unwrap().wait().unwrap();
}
