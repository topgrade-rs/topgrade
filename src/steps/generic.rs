#![allow(unused_imports)]

use std::path::PathBuf;
use std::process::Command;
use std::{env, path::Path};
use std::{fs, io::Write};

use color_eyre::eyre::eyre;
use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use tempfile::tempfile_in;
use tracing::{debug, error};

use crate::command::{CommandExt, Utf8Output};
use crate::execution_context::ExecutionContext;
use crate::executor::ExecutorOutput;
use crate::terminal::{print_separator, shell};
use crate::utils::{self, check_is_python_2_or_shim, require, require_option, which, PathExt, REQUIRE_SUDO};
use crate::Step;
use crate::HOME_DIR;
use crate::{
    error::{SkipStep, StepFailed, TopgradeError},
    terminal::print_warning,
};

pub fn run_cargo_update(ctx: &ExecutionContext) -> Result<()> {
    let cargo_dir = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| HOME_DIR.join(".cargo"))
        .require()?;
    require("cargo").or_else(|_| {
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
    let cargo_update = require("cargo-install-update")
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
        .status_checked()?;

    if ctx.config().cleanup() {
        let cargo_cache = require("cargo-cache")
            .ok()
            .or_else(|| cargo_dir.join("bin/cargo-cache").if_exists());
        match cargo_cache {
            Some(e) => {
                ctx.run_type().execute(e).args(["-a"]).status_checked()?;
            }
            None => {
                let message = String::from("cargo-cache isn't installed so Topgrade can't cleanup cargo packages.\nInstall cargo-cache by running `cargo install cargo-cache`");
                print_warning(message);
            }
        }
    }

    Ok(())
}

pub fn run_flutter_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let flutter = require("flutter")?;

    print_separator("Flutter");
    ctx.run_type().execute(flutter).arg("upgrade").status_checked()
}

pub fn run_gem(ctx: &ExecutionContext) -> Result<()> {
    let gem = require("gem")?;
    HOME_DIR.join(".gem").require()?;

    print_separator("Gems");

    let mut command = ctx.run_type().execute(gem);
    command.arg("update");

    if env::var_os("RBENV_SHELL").is_none() {
        debug!("Detected rbenv. Avoiding --user-install");
        command.arg("--user-install");
    }

    command.status_checked()
}

pub fn run_rubygems(ctx: &ExecutionContext) -> Result<()> {
    HOME_DIR.join(".gem").require()?;
    let gem = require("gem")?;

    print_separator("RubyGems");
    let gem_path_str = gem.as_os_str();
    if gem_path_str.to_str().unwrap().contains("asdf") {
        ctx.run_type()
            .execute(gem)
            .args(["update", "--system"])
            .status_checked()?;
    } else {
        let sudo = require_option(ctx.sudo().as_ref(), REQUIRE_SUDO.to_string())?;
        if !Path::new("/usr/lib/ruby/vendor_ruby/rubygems/defaults/operating_system.rb").exists() {
            ctx.run_type()
                .execute(sudo)
                .arg("-EH")
                .arg(gem)
                .args(["update", "--system"])
                .status_checked()?;
        }
    }

    Ok(())
}

pub fn run_haxelib_update(ctx: &ExecutionContext) -> Result<()> {
    let haxelib = require("haxelib")?;

    let haxelib_dir =
        PathBuf::from(std::str::from_utf8(&Command::new(&haxelib).arg("config").output_checked()?.stdout)?.trim())
            .require()?;

    let directory_writable = tempfile_in(&haxelib_dir).is_ok();
    debug!("{:?} writable: {}", haxelib_dir, directory_writable);

    print_separator("haxelib");

    let mut command = if directory_writable {
        ctx.run_type().execute(&haxelib)
    } else {
        let sudo = require_option(ctx.sudo().as_ref(), REQUIRE_SUDO.to_string())?;
        let mut c = ctx.run_type().execute(sudo);
        c.arg(&haxelib);
        c
    };

    command.arg("update").status_checked()
}

pub fn run_sheldon(ctx: &ExecutionContext) -> Result<()> {
    let sheldon = require("sheldon")?;

    print_separator("Sheldon");

    ctx.run_type()
        .execute(sheldon)
        .args(["lock", "--update"])
        .status_checked()
}

pub fn run_fossil(ctx: &ExecutionContext) -> Result<()> {
    let fossil = require("fossil")?;

    print_separator("Fossil");

    ctx.run_type().execute(fossil).args(["all", "sync"]).status_checked()
}

pub fn run_micro(ctx: &ExecutionContext) -> Result<()> {
    let micro = require("micro")?;

    print_separator("micro");

    let stdout = ctx
        .run_type()
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
pub fn run_apm(ctx: &ExecutionContext) -> Result<()> {
    let apm = require("apm")?;

    print_separator("Atom Package Manager");

    ctx.run_type()
        .execute(apm)
        .args(["upgrade", "--confirm=false"])
        .status_checked()
}

pub fn run_rustup(ctx: &ExecutionContext) -> Result<()> {
    let rustup = require("rustup")?;

    print_separator("rustup");
    ctx.run_type().execute(rustup).arg("update").status_checked()
}

pub fn run_juliaup(ctx: &ExecutionContext) -> Result<()> {
    let juliaup = require("juliaup")?;

    print_separator("juliaup");

    if juliaup.canonicalize()?.is_descendant_of(&HOME_DIR) {
        ctx.run_type()
            .execute(&juliaup)
            .args(["self", "update"])
            .status_checked()?;
    }

    ctx.run_type().execute(&juliaup).arg("update").status_checked()
}

pub fn run_choosenim(ctx: &ExecutionContext) -> Result<()> {
    let choosenim = require("choosenim")?;

    print_separator("choosenim");
    let run_type = ctx.run_type();

    run_type.execute(&choosenim).args(["update", "self"]).status_checked()?;
    run_type.execute(&choosenim).args(["update", "stable"]).status_checked()
}

pub fn run_krew_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let krew = require("kubectl-krew")?;

    print_separator("Krew");

    ctx.run_type().execute(krew).args(["upgrade"]).status_checked()
}

pub fn run_gcloud_components_update(ctx: &ExecutionContext) -> Result<()> {
    let gcloud = require("gcloud")?;

    if gcloud.starts_with("/snap") {
        Ok(())
    } else {
        print_separator("gcloud");

        ctx.run_type()
            .execute(gcloud)
            .args(["components", "update", "--quiet"])
            .status_checked()
    }
}

pub fn run_jetpack(ctx: &ExecutionContext) -> Result<()> {
    let jetpack = require("jetpack")?;

    print_separator("Jetpack");

    ctx.run_type()
        .execute(jetpack)
        .args(["global", "update"])
        .status_checked()
}

pub fn run_rtcl(ctx: &ExecutionContext) -> Result<()> {
    let rupdate = require("rupdate")?;

    print_separator("rtcl");

    ctx.run_type().execute(rupdate).status_checked()
}

pub fn run_opam_update(ctx: &ExecutionContext) -> Result<()> {
    let opam = require("opam")?;

    print_separator("OCaml Package Manager");

    ctx.run_type().execute(&opam).arg("update").status_checked()?;
    ctx.run_type().execute(&opam).arg("upgrade").status_checked()?;

    if ctx.config().cleanup() {
        ctx.run_type().execute(&opam).arg("clean").status_checked()?;
    }

    Ok(())
}

pub fn run_vcpkg_update(ctx: &ExecutionContext) -> Result<()> {
    let vcpkg = require("vcpkg")?;
    print_separator("vcpkg");

    #[cfg(unix)]
    let is_root_install = !&vcpkg.starts_with("/home");

    #[cfg(not(unix))]
    let is_root_install = false;

    let mut command = if is_root_install {
        ctx.run_type().execute(&vcpkg)
    } else {
        let sudo = require_option(ctx.sudo().as_ref(), REQUIRE_SUDO.to_string())?;
        let mut c = ctx.run_type().execute(sudo);
        c.arg(&vcpkg);
        c
    };

    command.args(["upgrade", "--no-dry-run"]).status_checked()
}

pub fn run_pipx_update(ctx: &ExecutionContext) -> Result<()> {
    let pipx = require("pipx")?;
    print_separator("pipx");

    ctx.run_type().execute(pipx).arg("upgrade-all").status_checked()
}

pub fn run_conda_update(ctx: &ExecutionContext) -> Result<()> {
    let conda = require("conda")?;

    let output = Command::new("conda")
        .args(["config", "--show", "auto_activate_base"])
        .output_checked_utf8()?;
    debug!("Conda output: {}", output.stdout);
    if output.stdout.contains("False") {
        return Err(SkipStep("auto_activate_base is set to False".to_string()).into());
    }

    print_separator("Conda");

    let mut command = ctx.run_type().execute(conda);
    command.args(["update", "--all", "-n", "base"]);
    if ctx.config().yes(Step::Conda) {
        command.arg("--yes");
    }
    command.status_checked()
}

pub fn run_mamba_update(ctx: &ExecutionContext) -> Result<()> {
    let mamba = require("mamba")?;

    let output = Command::new("mamba")
        .args(["config", "--show", "auto_activate_base"])
        .output_checked_utf8()?;
    debug!("Mamba output: {}", output.stdout);
    if output.stdout.contains("False") {
        return Err(SkipStep("auto_activate_base is set to False".to_string()).into());
    }

    print_separator("Mamba");

    let mut command = ctx.run_type().execute(mamba);
    command.args(["update", "--all", "-n", "base"]);
    if ctx.config().yes(Step::Mamba) {
        command.arg("--yes");
    }
    command.status_checked()
}

pub fn run_pip3_update(ctx: &ExecutionContext) -> Result<()> {
    let py = require("python").and_then(check_is_python_2_or_shim);
    let py3 = require("python3").and_then(check_is_python_2_or_shim);

    let python3 = match (py, py3) {
        // prefer `python` if it is available and is a valid Python 3.
        (Ok(py), _) => py,
        (Err(_), Ok(py3)) => py3,
        (Err(py_err), Err(py3_err)) => {
            return Err(SkipStep(format!("Skip due to following reasons: {} {}", py_err, py3_err)).into());
        }
    };

    Command::new(&python3)
        .args(["-m", "pip"])
        .output_checked_utf8()
        .map_err(|_| SkipStep("pip does not exists".to_string()))?;

    let check_externally_managed = "import sysconfig; from os import path; print('Y') if path.isfile(path.join(sysconfig.get_path('stdlib'), 'EXTERNALLY-MANAGED')) else print('N')";
    Command::new(&python3)
        .args(["-c", check_externally_managed])
        .output_checked_utf8()
        .map_err(|_| SkipStep("pip may be externally managed".to_string()))
        .and_then(|output| match output.stdout.trim() {
            "N" => Ok(()),
            "Y" => Err(SkipStep("pip is externally managed".to_string())),
            _ => {
                print_warning("Unexpected output when checking EXTERNALLY-MANAGED");
                print_warning(output.stdout.trim());
                Err(SkipStep("pip may be externally managed".to_string()))
            }
        })?;

    print_separator("pip3");
    if env::var("VIRTUAL_ENV").is_ok() {
        print_warning("This step is will be skipped when running inside a virtual environment");
        return Err(SkipStep("Does not run inside a virtual environment".to_string()).into());
    }

    ctx.run_type()
        .execute(&python3)
        .args(["-m", "pip", "install", "--upgrade", "--user", "pip"])
        .status_checked()
}

pub fn run_pip_review_update(ctx: &ExecutionContext) -> Result<()> {
    let pip_review = require("pip-review")?;

    print_separator("pip-review");

    if !ctx.config().enable_pip_review() {
        print_warning(
            "Pip-review is disabled by default. Enable it by setting enable_pip_review=true in the configuration.",
        );
        return Err(SkipStep(String::from("Pip-review is disabled by default")).into());
    }
    ctx.run_type()
        .execute(pip_review)
        .arg("--auto")
        .status_checked_with_codes(&[1])?;

    Ok(())
}
pub fn run_pip_review_local_update(ctx: &ExecutionContext) -> Result<()> {
    let pip_review = require("pip-review")?;

    print_separator("pip-review (local)");

    if !ctx.config().enable_pip_review_local() {
        print_warning(
            "Pip-review (local) is disabled by default. Enable it by setting enable_pip_review_local=true in the configuration.",
        );
        return Err(SkipStep(String::from("Pip-review (local) is disabled by default")).into());
    }
    ctx.run_type()
        .execute(pip_review)
        .arg("--local")
        .arg("--auto")
        .status_checked_with_codes(&[1])?;

    Ok(())
}
pub fn run_pipupgrade_update(ctx: &ExecutionContext) -> Result<()> {
    let pipupgrade = require("pipupgrade")?;

    print_separator("Pipupgrade");
    if !ctx.config().enable_pipupgrade() {
        print_warning(
            "Pipupgrade is disabled by default. Enable it by setting enable_pipupgrade=true in the configuration.",
        );
        return Err(SkipStep(String::from("Pipupgrade is disabled by default")).into());
    }
    ctx.run_type()
        .execute(pipupgrade)
        .args(ctx.config().pipupgrade_arguments().split_whitespace())
        .status_checked()?;

    Ok(())
}
pub fn run_stack_update(ctx: &ExecutionContext) -> Result<()> {
    if require("ghcup").is_ok() {
        // `ghcup` is present and probably(?) being used to install `stack`.
        // Don't upgrade `stack`, let `ghcup` handle it. Per `ghcup install stack`:
        // !!! Additionally, you should upgrade stack only through ghcup and not use 'stack upgrade' !!!
        return Ok(());
    }

    let stack = require("stack")?;
    print_separator("stack");

    ctx.run_type().execute(stack).arg("upgrade").status_checked()
}

pub fn run_ghcup_update(ctx: &ExecutionContext) -> Result<()> {
    let ghcup = require("ghcup")?;
    print_separator("ghcup");

    ctx.run_type().execute(ghcup).arg("upgrade").status_checked()
}

pub fn run_tlmgr_update(ctx: &ExecutionContext) -> Result<()> {
    cfg_if::cfg_if! {
        if #[cfg(any(target_os = "linux", target_os = "android"))] {
            if !ctx.config().enable_tlmgr_linux() {
                return Err(SkipStep(String::from("tlmgr must be explicity enabled in the configuration to run in Android/Linux")).into());
            }
        }
    }

    let tlmgr = require("tlmgr")?;
    let kpsewhich = require("kpsewhich")?;
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
        let sudo = require_option(ctx.sudo().as_ref(), REQUIRE_SUDO.to_string())?;
        let mut c = ctx.run_type().execute(sudo);
        c.arg(&tlmgr);
        c
    };
    command.args(["update", "--self", "--all"]);

    command.status_checked()
}

pub fn run_chezmoi_update(ctx: &ExecutionContext) -> Result<()> {
    let chezmoi = require("chezmoi")?;
    HOME_DIR.join(".local/share/chezmoi").require()?;

    print_separator("chezmoi");

    ctx.run_type().execute(chezmoi).arg("update").status_checked()
}

pub fn run_myrepos_update(ctx: &ExecutionContext) -> Result<()> {
    let myrepos = require("mr")?;
    HOME_DIR.join(".mrconfig").require()?;

    print_separator("myrepos");

    ctx.run_type()
        .execute(&myrepos)
        .arg("--directory")
        .arg(&*HOME_DIR)
        .arg("checkout")
        .status_checked()?;
    ctx.run_type()
        .execute(&myrepos)
        .arg("--directory")
        .arg(&*HOME_DIR)
        .arg("update")
        .status_checked()
}

pub fn run_custom_command(name: &str, command: &str, ctx: &ExecutionContext) -> Result<()> {
    print_separator(name);
    let mut exec = ctx.run_type().execute(shell());
    #[cfg(unix)]
    let command = if let Some(command) = command.strip_prefix("-i ") {
        exec.arg("-i");
        command
    } else {
        command
    };
    exec.arg("-c").arg(command).status_checked()
}

pub fn run_composer_update(ctx: &ExecutionContext) -> Result<()> {
    let composer = require("composer")?;
    let composer_home = Command::new(&composer)
        .args(["global", "config", "--absolute", "--quiet", "home"])
        .output_checked_utf8()
        .map_err(|e| (SkipStep(format!("Error getting the composer directory: {e}"))))
        .map(|s| PathBuf::from(s.stdout.trim()))?
        .require()?;

    if !composer_home.is_descendant_of(&HOME_DIR) {
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
                    let sudo = require_option(ctx.sudo().as_ref(), REQUIRE_SUDO.to_string())?;
                    ctx.run_type()
                        .execute(sudo)
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
            if let Some(valet) = which("valet") {
                ctx.run_type().execute(valet).arg("install").status_checked()?;
            }
        }
    }

    Ok(())
}

pub fn run_dotnet_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let dotnet = require("dotnet")?;

    // Skip when the `dotnet tool list` subcommand fails.
    // (This is expected when a dotnet runtime is installed but no SDK.)
    let output = match ctx
        .run_type()
        .execute(&dotnet)
        .args(["tool", "list", "--global"])
        .output_checked_utf8()
    {
        Ok(output) => output,
        Err(_) => {
            return Err(SkipStep(String::from(
                "Error running `dotnet tool list`. This is expected when a dotnet runtime is installed but no SDK.",
            ))
            .into())
        }
    };

    let mut packages = output
        .stdout
        .lines()
        // Skip the header:
        //
        // Package Id      Version      Commands
        // -------------------------------------
        //
        // One thing to note is that .NET SDK respect locale, which means this
        // header can be printed in languages other than English, do NOT use it
        // to do any check.
        .skip(2)
        .filter(|line| !line.is_empty())
        .peekable();

    if packages.peek().is_none() {
        return Err(SkipStep(String::from("No dotnet global tools installed")).into());
    }

    print_separator(".NET");

    for package in packages {
        let package_name = package.split_whitespace().next().unwrap();
        ctx.run_type()
            .execute(&dotnet)
            .args(["tool", "update", package_name, "--global"])
            .status_checked()
            .with_context(|| format!("Failed to update .NET package {package_name}"))?;
    }

    Ok(())
}

pub fn run_helix_grammars(ctx: &ExecutionContext) -> Result<()> {
    require("helix")?;

    print_separator("Helix");

    let sudo = require_option(ctx.sudo().as_ref(), REQUIRE_SUDO.to_string())?;
    ctx.run_type()
        .execute(sudo)
        .args(["helix", "--grammar", "fetch"])
        .status_checked()
        .with_context(|| "Failed to download helix grammars!")?;

    ctx.run_type()
        .execute(sudo)
        .args(["helix", "--grammar", "build"])
        .status_checked()
        .with_context(|| "Failed to build helix grammars!")?;

    Ok(())
}

pub fn run_raco_update(ctx: &ExecutionContext) -> Result<()> {
    let raco = require("raco")?;

    print_separator("Racket Package Manager");

    ctx.run_type()
        .execute(raco)
        .args(["pkg", "update", "--all"])
        .status_checked()
}

pub fn bin_update(ctx: &ExecutionContext) -> Result<()> {
    let bin = require("bin")?;

    print_separator("Bin");
    ctx.run_type().execute(bin).arg("update").status_checked()
}

pub fn spicetify_upgrade(ctx: &ExecutionContext) -> Result<()> {
    // As of 04-07-2023 NixOS packages Spicetify with the `spicetify-cli` binary name
    let spicetify = require("spicetify").or(require("spicetify-cli"))?;

    print_separator("Spicetify");
    ctx.run_type().execute(spicetify).arg("upgrade").status_checked()
}

pub fn run_ghcli_extensions_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let gh = require("gh")?;
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
    let julia = require("julia")?;

    print_separator("Julia Packages");

    ctx.run_type()
        .execute(julia)
        .args(["-e", "using Pkg; Pkg.update()"])
        .status_checked()
}

pub fn run_helm_repo_update(ctx: &ExecutionContext) -> Result<()> {
    let helm = require("helm")?;

    print_separator("Helm");

    let no_repo = "no repositories found";
    let mut success = true;
    let mut exec = ctx.run_type().execute(helm);
    if let Err(e) = exec.arg("repo").arg("update").status_checked() {
        error!("Updating repositories failed: {}", e);
        success = match exec.output_checked_utf8() {
            Ok(s) => s.stdout.contains(no_repo) || s.stderr.contains(no_repo),
            Err(e) => match e.downcast_ref::<TopgradeError>() {
                Some(TopgradeError::ProcessFailedWithOutput(_, _, stderr)) => stderr.contains(no_repo),
                _ => false,
            },
        };
    }

    if success {
        Ok(())
    } else {
        Err(eyre!(StepFailed))
    }
}

pub fn run_stew(ctx: &ExecutionContext) -> Result<()> {
    let stew = require("stew")?;

    print_separator("stew");
    ctx.run_type().execute(stew).args(["upgrade", "--all"]).status_checked()
}

pub fn run_bob(ctx: &ExecutionContext) -> Result<()> {
    let bob = require("bob")?;

    print_separator("Bob");

    ctx.run_type().execute(bob).args(["update", "--all"]).status_checked()
}
