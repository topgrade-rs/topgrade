#![allow(unused_imports)]

use std::path::PathBuf;
use std::process::Command;
use std::{env, path::Path};
use std::{fs, io::Write};

use color_eyre::eyre::eyre;
use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use directories::BaseDirs;
use tempfile::tempfile_in;
use tracing::debug;

use crate::command::{CommandExt, Utf8Output};
use crate::execution_context::ExecutionContext;
use crate::executor::{ExecutorOutput, RunType};
use crate::terminal::{print_separator, shell};
use crate::utils::{self, require_option, PathExt};
use crate::{
    error::{SkipStep, TopgradeError},
    terminal::print_warning,
};

pub fn run_cargo_update(ctx: &ExecutionContext) -> Result<()> {
    let cargo_dir = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.base_dirs().home_dir().join(".cargo"))
        .require()?;
    utils::require("cargo").or_else(|_| {
        require_option(
            cargo_dir.join("bin/cargo").if_exists(),
            String::from("No cargo detected"),
        )
    })?;

    let toml_file = cargo_dir.join(".crates.toml").require()?;

    if fs::metadata(&toml_file)?.len() == 0 {
        return Err(SkipStep(format!("{} exists but empty", &toml_file.display())).into());
    }

    print_separator("Cargo");
    let cargo_update = utils::require("cargo-install-update")
        .ok()
        .or_else(|| cargo_dir.join("bin/cargo-install-update").if_exists());
    let cargo_update = match cargo_update {
        Some(e) => e,
        None => {
            let message = String::from("cargo-update isn't installed so Topgrade can't upgrade cargo packages.\nInstall cargo-update by running `cargo install cargo-update`");
            print_warning(&message);
            return Err(SkipStep(message).into());
        }
    };

    ctx.run_type()
        .execute(cargo_update)
        .args(["install-update", "--git", "--all"])
        .status_checked()
}

pub fn run_flutter_upgrade(run_type: RunType) -> Result<()> {
    let flutter = utils::require("flutter")?;

    print_separator("Flutter");
    run_type.execute(flutter).arg("upgrade").status_checked()
}

pub fn run_gem(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let gem = utils::require("gem")?;
    base_dirs.home_dir().join(".gem").require()?;

    print_separator("Gems");

    let mut command = run_type.execute(gem);
    command.arg("update");

    if env::var_os("RBENV_SHELL").is_none() {
        debug!("Detected rbenv. Avoiding --user-install");
        command.arg("--user-install");
    }

    command.status_checked()
}

pub fn run_rubygems(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let gem = utils::require("gem")?;
    base_dirs.home_dir().join(".gem").require()?;

    print_separator("RubyGems");

    let mut command = run_type.execute(gem);
    command.args(["update", "--system"]);

    if env::var_os("RBENV_SHELL").is_none() {
        debug!("Detected rbenv. Avoiding --user-install");
        command.arg("--user-install");
    }

    command.status_checked()
}

pub fn run_haxelib_update(ctx: &ExecutionContext) -> Result<()> {
    let haxelib = utils::require("haxelib")?;

    let haxelib_dir =
        PathBuf::from(std::str::from_utf8(&Command::new(&haxelib).arg("config").output_checked()?.stdout)?.trim())
            .require()?;

    let directory_writable = tempfile_in(&haxelib_dir).is_ok();
    debug!("{:?} writable: {}", haxelib_dir, directory_writable);

    print_separator("haxelib");

    let mut command = if directory_writable {
        ctx.run_type().execute(&haxelib)
    } else {
        let mut c = ctx
            .run_type()
            .execute(ctx.sudo().as_ref().ok_or(TopgradeError::SudoRequired)?);
        c.arg(&haxelib);
        c
    };

    command.arg("update").status_checked()
}

pub fn run_sheldon(ctx: &ExecutionContext) -> Result<()> {
    let sheldon = utils::require("sheldon")?;

    print_separator("Sheldon");

    ctx.run_type()
        .execute(sheldon)
        .args(["lock", "--update"])
        .status_checked()
}

pub fn run_fossil(run_type: RunType) -> Result<()> {
    let fossil = utils::require("fossil")?;

    print_separator("Fossil");

    run_type.execute(fossil).args(["all", "sync"]).status_checked()
}

pub fn run_micro(run_type: RunType) -> Result<()> {
    let micro = utils::require("micro")?;

    print_separator("micro");

    let stdout = run_type
        .execute(micro)
        .args(["-plugin", "update"])
        .output_checked_utf8()?
        .stdout;
    std::io::stdout().write_all(stdout.as_bytes())?;

    if stdout.contains("Nothing to install / update") || stdout.contains("One or more plugins installed") {
        Ok(())
    } else {
        Err(eyre!("micro output does not indicate success: {}", stdout))
    }
}

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
)))]
pub fn run_apm(run_type: RunType) -> Result<()> {
    let apm = utils::require("apm")?;

    print_separator("Atom Package Manager");

    run_type
        .execute(apm)
        .args(["upgrade", "--confirm=false"])
        .status_checked()
}

pub fn run_rustup(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let rustup = utils::require("rustup")?;

    print_separator("rustup");

    if rustup.canonicalize()?.is_descendant_of(base_dirs.home_dir()) {
        run_type.execute(&rustup).args(["self", "update"]).status_checked()?;
    }

    run_type.execute(&rustup).arg("update").status_checked()
}

pub fn run_juliaup(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let juliaup = utils::require("juliaup")?;

    print_separator("juliaup");

    if juliaup.canonicalize()?.is_descendant_of(base_dirs.home_dir()) {
        run_type.execute(&juliaup).args(["self", "update"]).status_checked()?;
    }

    run_type.execute(&juliaup).arg("update").status_checked()
}

pub fn run_choosenim(ctx: &ExecutionContext) -> Result<()> {
    let choosenim = utils::require("choosenim")?;

    print_separator("choosenim");
    let run_type = ctx.run_type();

    run_type.execute(&choosenim).args(["update", "self"]).status_checked()?;
    run_type.execute(&choosenim).args(["update", "stable"]).status_checked()
}

pub fn run_krew_upgrade(run_type: RunType) -> Result<()> {
    let krew = utils::require("kubectl-krew")?;

    print_separator("Krew");

    run_type.execute(krew).args(["upgrade"]).status_checked()
}

pub fn run_gcloud_components_update(run_type: RunType) -> Result<()> {
    let gcloud = utils::require("gcloud")?;

    if gcloud.starts_with("/snap") {
        Ok(())
    } else {
        print_separator("gcloud");

        run_type
            .execute(gcloud)
            .args(["components", "update", "--quiet"])
            .status_checked()
    }
}

pub fn run_jetpack(run_type: RunType) -> Result<()> {
    let jetpack = utils::require("jetpack")?;

    print_separator("Jetpack");

    run_type.execute(jetpack).args(["global", "update"]).status_checked()
}

pub fn run_rtcl(ctx: &ExecutionContext) -> Result<()> {
    let rupdate = utils::require("rupdate")?;

    print_separator("rtcl");

    ctx.run_type().execute(rupdate).status_checked()
}

pub fn run_opam_update(ctx: &ExecutionContext) -> Result<()> {
    let opam = utils::require("opam")?;

    print_separator("OCaml Package Manager");

    ctx.run_type().execute(&opam).arg("update").status_checked()?;
    ctx.run_type().execute(&opam).arg("upgrade").status_checked()?;

    if ctx.config().cleanup() {
        ctx.run_type().execute(&opam).arg("clean").status_checked()?;
    }

    Ok(())
}

pub fn run_vcpkg_update(run_type: RunType) -> Result<()> {
    let vcpkg = utils::require("vcpkg")?;
    print_separator("vcpkg");

    run_type
        .execute(vcpkg)
        .args(["upgrade", "--no-dry-run"])
        .status_checked()
}

pub fn run_pipx_update(run_type: RunType) -> Result<()> {
    let pipx = utils::require("pipx")?;
    print_separator("pipx");

    run_type.execute(pipx).arg("upgrade-all").status_checked()
}

pub fn run_conda_update(ctx: &ExecutionContext) -> Result<()> {
    let conda = utils::require("conda")?;

    let output = Command::new("conda")
        .args(["config", "--show", "auto_activate_base"])
        .output_checked_utf8()?;
    debug!("Conda output: {}", output.stdout);
    if output.stdout.contains("False") {
        return Err(SkipStep("auto_activate_base is set to False".to_string()).into());
    }

    print_separator("Conda");

    ctx.run_type()
        .execute(conda)
        .args(["update", "--all", "-y"])
        .status_checked()
}

pub fn run_pip3_update(run_type: RunType) -> Result<()> {
    let python3 = utils::require("python3")?;
    Command::new(&python3)
        .args(["-m", "pip"])
        .output_checked_utf8()
        .map_err(|_| SkipStep("pip does not exists".to_string()))?;

    print_separator("pip3");
    if std::env::var("VIRTUAL_ENV").is_ok() {
        print_warning("This step is will be skipped when running inside a virtual environment");
        return Err(SkipStep("Does not run inside a virtual environment".to_string()).into());
    }

    run_type
        .execute(&python3)
        .args(["-m", "pip", "install", "--upgrade", "--user", "pip"])
        .status_checked()
}

pub fn run_stack_update(run_type: RunType) -> Result<()> {
    if utils::require("ghcup").is_ok() {
        // `ghcup` is present and probably(?) being used to install `stack`.
        // Don't upgrade `stack`, let `ghcup` handle it. Per `ghcup install stack`:
        // !!! Additionally, you should upgrade stack only through ghcup and not use 'stack upgrade' !!!
        return Ok(());
    }

    let stack = utils::require("stack")?;
    print_separator("stack");

    run_type.execute(stack).arg("upgrade").status_checked()
}

pub fn run_ghcup_update(run_type: RunType) -> Result<()> {
    let ghcup = utils::require("ghcup")?;
    print_separator("ghcup");

    run_type.execute(ghcup).arg("upgrade").status_checked()
}

pub fn run_tlmgr_update(ctx: &ExecutionContext) -> Result<()> {
    cfg_if::cfg_if! {
        if #[cfg(any(target_os = "linux", target_os = "android"))] {
            if !ctx.config().enable_tlmgr_linux() {
                return Err(SkipStep(String::from("tlmgr must be explicity enabled in the configuration to run in Android/Linux")).into());
            }
        }
    }

    let tlmgr = utils::require("tlmgr")?;
    let kpsewhich = utils::require("kpsewhich")?;
    let tlmgr_directory = {
        let mut d = PathBuf::from(
            &Command::new(kpsewhich)
                .arg("-var-value=SELFAUTOPARENT")
                .output_checked_utf8()?
                .stdout
                .trim(),
        );
        d.push("tlpkg");
        d
    }
    .require()?;

    let directory_writable = tempfile_in(&tlmgr_directory).is_ok();
    debug!("{:?} writable: {}", tlmgr_directory, directory_writable);

    print_separator("TeX Live package manager");

    let mut command = if directory_writable {
        ctx.run_type().execute(&tlmgr)
    } else {
        let mut c = ctx
            .run_type()
            .execute(ctx.sudo().as_ref().ok_or(TopgradeError::SudoRequired)?);
        c.arg(&tlmgr);
        c
    };
    command.args(["update", "--self", "--all"]);

    command.status_checked()
}

pub fn run_chezmoi_update(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let chezmoi = utils::require("chezmoi")?;
    base_dirs.home_dir().join(".local/share/chezmoi").require()?;

    print_separator("chezmoi");

    run_type.execute(chezmoi).arg("update").status_checked()
}

pub fn run_myrepos_update(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let myrepos = utils::require("mr")?;
    base_dirs.home_dir().join(".mrconfig").require()?;

    print_separator("myrepos");

    run_type
        .execute(&myrepos)
        .arg("--directory")
        .arg(base_dirs.home_dir())
        .arg("checkout")
        .status_checked()?;
    run_type
        .execute(&myrepos)
        .arg("--directory")
        .arg(base_dirs.home_dir())
        .arg("update")
        .status_checked()
}

pub fn run_custom_command(name: &str, command: &str, ctx: &ExecutionContext) -> Result<()> {
    print_separator(name);
    ctx.run_type().execute(shell()).arg("-c").arg(command).status_checked()
}

pub fn run_composer_update(ctx: &ExecutionContext) -> Result<()> {
    let composer = utils::require("composer")?;
    let composer_home = Command::new(&composer)
        .args(["global", "config", "--absolute", "--quiet", "home"])
        .output_checked_utf8()
        .map_err(|e| (SkipStep(format!("Error getting the composer directory: {}", e))))
        .map(|s| PathBuf::from(s.stdout.trim()))?
        .require()?;

    if !composer_home.is_descendant_of(ctx.base_dirs().home_dir()) {
        return Err(SkipStep(format!(
            "Composer directory {} isn't a decandent of the user's home directory",
            composer_home.display()
        ))
        .into());
    }

    print_separator("Composer");

    if ctx.config().composer_self_update() {
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                // If self-update fails without sudo then there's probably an update
                let has_update = match ctx.run_type().execute(&composer).arg("self-update").output()? {
                    ExecutorOutput::Wet(output) => !output.status.success(),
                    _ => false
                };

                if has_update {
                    ctx.run_type()
                        .execute(ctx.sudo().as_ref().unwrap())
                        .arg(&composer)
                        .arg("self-update")
                        .status_checked()?;
                }
            } else {
                ctx.run_type().execute(&composer).arg("self-update").status_checked()?;
            }
        }
    }

    let output = ctx.run_type().execute(&composer).args(["global", "update"]).output()?;
    if let ExecutorOutput::Wet(output) = output {
        let output: Utf8Output = output.try_into()?;
        print!("{}\n{}", output.stdout, output.stderr);
        if output.stdout.contains("valet") || output.stderr.contains("valet") {
            if let Some(valet) = utils::which("valet") {
                ctx.run_type().execute(valet).arg("install").status_checked()?;
            }
        }
    }

    Ok(())
}

pub fn run_dotnet_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let dotnet = utils::require("dotnet")?;

    let output = Command::new(dotnet)
        .args(["tool", "list", "--global"])
        .output_checked_utf8()?;

    if !output.stdout.starts_with("Package Id") {
        return Err(SkipStep(String::from("dotnet did not output packages")).into());
    }

    let mut packages = output.stdout.lines().skip(2).filter(|line| !line.is_empty()).peekable();

    if packages.peek().is_none() {
        return Err(SkipStep(String::from("No dotnet global tools installed")).into());
    }

    print_separator(".NET");

    for package in packages {
        let package_name = package.split_whitespace().next().unwrap();
        ctx.run_type()
            .execute("dotnet")
            .args(["tool", "update", package_name, "--global"])
            .status_checked()
            .with_context(|| format!("Failed to update .NET package {package_name}"))?;
    }

    Ok(())
}

pub fn run_raco_update(run_type: RunType) -> Result<()> {
    let raco = utils::require("raco")?;

    print_separator("Racket Package Manager");

    run_type.execute(raco).args(["pkg", "update", "--all"]).status_checked()
}

pub fn bin_update(ctx: &ExecutionContext) -> Result<()> {
    let bin = utils::require("bin")?;

    print_separator("Bin");
    ctx.run_type().execute(bin).arg("update").status_checked()
}

pub fn spicetify_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let spicetify = utils::require("spicetify")?;

    print_separator("Spicetify");
    ctx.run_type().execute(spicetify).arg("upgrade").status_checked()
}

pub fn run_ghcli_extensions_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let gh = utils::require("gh")?;
    let result = Command::new(&gh).args(["extensions", "list"]).output_checked_utf8();
    if result.is_err() {
        debug!("GH result {:?}", result);
        return Err(SkipStep(String::from("GH failed")).into());
    }

    print_separator("GitHub CLI Extensions");
    ctx.run_type()
        .execute(&gh)
        .args(["extension", "upgrade", "--all"])
        .status_checked()
}

pub fn update_julia_packages(ctx: &ExecutionContext) -> Result<()> {
    let julia = utils::require("julia")?;

    print_separator("Julia Packages");

    ctx.run_type()
        .execute(julia)
        .args(["-e", "using Pkg; Pkg.update()"])
        .status_checked()
}
