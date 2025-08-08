use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use color_eyre::eyre::{eyre, OptionExt};
use jetbrains_toolbox_updater::{find_jetbrains_toolbox, update_jetbrains_toolbox, FindError};
use regex::bytes::Regex;
use rust_i18n::t;
use semver::Version;
use std::ffi::OsString;
use std::iter::once;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;
use std::{env, path::Path};
use std::{fs, io::Write};
use tempfile::tempfile_in;
use tracing::{debug, error, warn};

use crate::command::{CommandExt, Utf8Output};
use crate::execution_context::ExecutionContext;
use crate::executor::ExecutorOutput;
use crate::output_changed_message;
use crate::step::Step;
use crate::sudo::SudoExecuteOpts;
use crate::terminal::{print_separator, shell};
use crate::utils::{check_is_python_2_or_shim, require, require_one, require_option, which, PathExt};
use crate::HOME_DIR;
use crate::{
    error::{SkipStep, StepFailed, TopgradeError},
    terminal::print_warning,
};

#[cfg(target_os = "linux")]
pub fn is_wsl() -> Result<bool> {
    let output = Command::new("uname").arg("-r").output_checked_utf8()?.stdout;
    debug!("Uname output: {}", output);
    Ok(output.contains("microsoft"))
}

#[cfg(not(target_os = "linux"))]
pub fn is_wsl() -> Result<bool> {
    Ok(false)
}

pub fn run_cargo_update(ctx: &ExecutionContext) -> Result<()> {
    let cargo_dir = env::var_os("CARGO_HOME")
        .map_or_else(|| HOME_DIR.join(".cargo"), PathBuf::from)
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

    let Some(cargo_update) = cargo_update else {
        let message = String::from("cargo-update isn't installed so Topgrade can't upgrade cargo packages.\nInstall cargo-update by running `cargo install cargo-update`");
        print_warning(&message);
        return Err(SkipStep(message).into());
    };

    ctx.execute(cargo_update)
        .args(["install-update", "--git", "--all"])
        .status_checked()?;

    if ctx.config().cleanup() {
        let cargo_cache = require("cargo-cache")
            .ok()
            .or_else(|| cargo_dir.join("bin/cargo-cache").if_exists());
        if let Some(e) = cargo_cache {
            ctx.execute(e).args(["-a"]).status_checked()?;
        } else {
            let message = String::from("cargo-cache isn't installed so Topgrade can't cleanup cargo packages.\nInstall cargo-cache by running `cargo install cargo-cache`");
            print_warning(message);
        }
    }

    Ok(())
}

pub fn run_flutter_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let flutter = require("flutter")?;

    print_separator("Flutter");
    ctx.execute(flutter).arg("upgrade").status_checked()
}

pub fn run_gem(ctx: &ExecutionContext) -> Result<()> {
    let gem = require("gem")?;
    HOME_DIR.join(".gem").require()?;

    print_separator("Gems");

    let mut command = ctx.execute(gem);
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
    if gem_path_str.to_str().unwrap().contains("asdf")
        || gem_path_str.to_str().unwrap().contains("mise")
        || gem_path_str.to_str().unwrap().contains(".rbenv")
        || gem_path_str.to_str().unwrap().contains(".rvm")
    {
        ctx.execute(gem).args(["update", "--system"]).status_checked()?;
    } else {
        let sudo = ctx.require_sudo()?;
        if !Path::new("/usr/lib/ruby/vendor_ruby/rubygems/defaults/operating_system.rb").exists() {
            sudo.execute_opts(ctx, &gem, SudoExecuteOpts::new().preserve_env().set_home())?
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
        ctx.execute(&haxelib)
    } else {
        let sudo = ctx.require_sudo()?;
        sudo.execute(ctx, &haxelib)?
    };

    command.arg("update").status_checked()
}

pub fn run_sheldon(ctx: &ExecutionContext) -> Result<()> {
    let sheldon = require("sheldon")?;

    print_separator("Sheldon");

    ctx.execute(sheldon).args(["lock", "--update"]).status_checked()
}

pub fn run_fossil(ctx: &ExecutionContext) -> Result<()> {
    let fossil = require("fossil")?;

    print_separator("Fossil");

    ctx.execute(fossil).args(["all", "sync"]).status_checked()
}

pub fn run_micro(ctx: &ExecutionContext) -> Result<()> {
    let micro = require("micro")?;

    print_separator("micro");

    let stdout = ctx
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

    ctx.execute(apm).args(["upgrade", "--confirm=false"]).status_checked()
}

enum Aqua {
    JetBrainsAqua(PathBuf),
    AquaCLI(PathBuf),
}

impl Aqua {
    fn aqua_cli(self) -> Result<PathBuf> {
        match self {
            Aqua::AquaCLI(aqua) => Ok(aqua),
            Aqua::JetBrainsAqua(_) => {
                Err(SkipStep("Command `aqua` probably points to JetBrains Aqua".to_string()).into())
            }
        }
    }

    fn jetbrains_aqua(self) -> Result<PathBuf> {
        match self {
            Aqua::JetBrainsAqua(path) => Ok(path),
            Aqua::AquaCLI(_) => Err(SkipStep("Command `aqua` probably points to Aqua CLI".to_string()).into()),
        }
    }
}

fn get_aqua(ctx: &ExecutionContext) -> Result<Aqua> {
    let aqua = require("aqua")?;

    // Check if `aqua --help` mentions "aqua". JetBrains Aqua does not, Aqua CLI does.
    let output = ctx.execute(&aqua).arg("--help").output_checked()?;

    if String::from_utf8(output.stdout)?.contains("aqua") {
        debug!("Detected `aqua` as Aqua CLI");
        Ok(Aqua::AquaCLI(aqua))
    } else {
        debug!("Detected `aqua` as JetBrains Aqua");
        Ok(Aqua::JetBrainsAqua(aqua))
    }
}

pub fn run_aqua(ctx: &ExecutionContext) -> Result<()> {
    let aqua = get_aqua(ctx)?.aqua_cli()?;

    print_separator("Aqua");
    if ctx.run_type().dry() {
        println!("{}", t!("Updating aqua ..."));
        println!("{}", t!("Updating aqua installed cli tools ..."));
        Ok(())
    } else {
        ctx.execute(&aqua).arg("update-aqua").status_checked()?;
        ctx.execute(&aqua).arg("update").status_checked()
    }
}

pub fn run_rustup(ctx: &ExecutionContext) -> Result<()> {
    let rustup = require("rustup")?;

    print_separator("rustup");
    ctx.execute(rustup).arg("update").status_checked()
}

pub fn run_rye(ctx: &ExecutionContext) -> Result<()> {
    let rye = require("rye")?;

    print_separator("Rye");
    ctx.execute(rye).args(["self", "update"]).status_checked()
}

pub fn run_elan(ctx: &ExecutionContext) -> Result<()> {
    let elan = require("elan")?;

    print_separator("elan");

    let disabled_error_msg = "self-update is disabled";
    let executor_output = ctx.execute(&elan).args(["self", "update"]).output()?;
    match executor_output {
        ExecutorOutput::Wet(command_output) => {
            if command_output.status.success() {
                // Flush the captured output
                std::io::stdout().lock().write_all(&command_output.stdout).unwrap();
                std::io::stderr().lock().write_all(&command_output.stderr).unwrap();
            } else {
                let stderr_as_str = std::str::from_utf8(&command_output.stderr).unwrap();
                if stderr_as_str.contains(disabled_error_msg) {
                    // `elan` is externally managed, we cannot do the update. Users
                    // won't see any error message because Topgrade captures them
                    // all.
                } else {
                    // `elan` is NOT externally managed, `elan self update` can
                    // be performed, but the invocation failed, so we report the
                    // error to the user and error out.
                    std::io::stdout().lock().write_all(&command_output.stdout).unwrap();
                    std::io::stderr().lock().write_all(&command_output.stderr).unwrap();

                    return Err(StepFailed.into());
                }
            }
        }
        ExecutorOutput::Dry => { /* nothing needed because in a dry run */ }
    }

    ctx.execute(&elan).arg("update").status_checked()
}

pub fn run_juliaup(ctx: &ExecutionContext) -> Result<()> {
    let juliaup = require("juliaup")?;

    print_separator("juliaup");

    if juliaup.canonicalize()?.is_descendant_of(&HOME_DIR) {
        ctx.execute(&juliaup).args(["self", "update"]).status_checked()?;
    }

    ctx.execute(&juliaup).arg("update").status_checked()?;

    if ctx.config().cleanup() {
        ctx.execute(&juliaup).arg("gc").status_checked()?;
    }

    Ok(())
}

pub fn run_choosenim(ctx: &ExecutionContext) -> Result<()> {
    let choosenim = require("choosenim")?;

    print_separator("choosenim");

    ctx.execute(&choosenim).args(["update", "self"]).status_checked()?;
    ctx.execute(&choosenim).args(["update", "stable"]).status_checked()
}

pub fn run_krew_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let krew = require("kubectl-krew")?;

    print_separator("Krew");

    ctx.execute(krew).args(["upgrade"]).status_checked()
}

pub fn run_gcloud_components_update(ctx: &ExecutionContext) -> Result<()> {
    let gcloud = require("gcloud")?;

    if gcloud.starts_with("/snap") {
        return Ok(());
    }

    print_separator("gcloud");

    let output = ctx
        .execute(&gcloud)
        .args(["components", "update", "--quiet"])
        .output()?;

    let output = match output {
        ExecutorOutput::Wet(wet) => wet,
        ExecutorOutput::Dry => return Ok(()),
    };

    // When `gcloud` is installed via `apt`, the components update via `apt` as well
    let stderr = String::from_utf8(output.stderr)?;
    if stderr.contains("You cannot perform this action because the Google Cloud CLI component manager")
        && stderr.contains("is disabled for this installation")
    {
        return Err(
            SkipStep("The Google Cloud CLI component manager is disabled for this installation".to_string()).into(),
        );
    }

    // Flush captured output
    std::io::stdout().write_all(&output.stdout)?;
    std::io::stderr().write_all(stderr.as_bytes())?;

    if !output.status.success() {
        return Err(eyre!("gcloud component update failed"));
    }

    Ok(())
}

pub fn run_jetpack(ctx: &ExecutionContext) -> Result<()> {
    let jetpack = require("jetpack")?;

    print_separator("Jetpack");

    ctx.execute(jetpack).args(["global", "update"]).status_checked()
}

pub fn run_rtcl(ctx: &ExecutionContext) -> Result<()> {
    let rupdate = require("rupdate")?;

    print_separator("rtcl");

    ctx.execute(rupdate).status_checked()
}

pub fn run_opam_update(ctx: &ExecutionContext) -> Result<()> {
    let opam = require("opam")?;

    print_separator("OCaml Package Manager");

    ctx.execute(&opam).arg("update").status_checked()?;

    let mut command = ctx.execute(&opam);
    command.arg("upgrade");
    if ctx.config().yes(Step::Opam) {
        command.arg("--yes");
    }
    command.status_checked()?;

    if ctx.config().cleanup() {
        ctx.execute(&opam).arg("clean").status_checked()?;
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
        ctx.execute(&vcpkg)
    } else {
        let sudo = ctx.require_sudo()?;
        sudo.execute(ctx, &vcpkg)?
    };

    command.args(["upgrade", "--no-dry-run"]).status_checked()
}

/// This functions runs for both VSCode and VSCodium, as most of the process is the same for both.
fn run_vscode_compatible<const VSCODIUM: bool>(ctx: &ExecutionContext) -> Result<()> {
    // Calling VSCode/VSCodium in WSL may install a server instead of updating extensions (https://github.com/topgrade-rs/topgrade/issues/594#issuecomment-1782157367)
    if is_wsl()? {
        return Err(SkipStep(String::from("Should not run in WSL")).into());
    }

    let name = if VSCODIUM { "VSCodium" } else { "VSCode" };
    let bin_name = if VSCODIUM { "codium" } else { "code" };
    let bin = require(bin_name)?;

    // VSCode has update command only since 1.86 version ("january 2024" update), disable the update for prior versions
    // Use command `code --version` which returns 3 lines: version, git commit, instruction set. We parse only the first one
    //
    // This should apply to VSCodium as well.
    let version: Result<Version> = match Command::new(&bin)
        .arg("--version")
        .output_checked_utf8()?
        .stdout
        .lines()
        .next()
    {
        Some(item) => {
            // Strip leading zeroes because `semver` does not allow them, but VSCodium uses them sometimes.
            //  This is not the case for VSCode, but just in case, and it can't really cause any issues.
            let item = item
                .split('.')
                .map(|s| if s == "0" { "0" } else { s.trim_start_matches('0') })
                .collect::<Vec<_>>()
                .join(".");
            Version::parse(&item).map_err(std::convert::Into::into)
        }
        None => {
            return Err(eyre!(output_changed_message!(
                &format!("{bin_name} --version"),
                "No first line"
            )))
        }
    };

    // Raise any errors in parsing the version
    //  The benefit of handling VSCodium versions so old that the version format is something
    //  unexpected is outweighed by the benefit of failing fast on new breaking versions
    let version =
        version.wrap_err_with(|| output_changed_message!(&format!("{bin_name} --version"), "Invalid version"))?;
    debug!("Detected {name} version as: {version}");

    if version < Version::new(1, 86, 0) {
        return Err(SkipStep(format!("Too old {name} version to have update extensions command")).into());
    }

    print_separator(if VSCODIUM {
        "VSCodium extensions"
    } else {
        "Visual Studio Code extensions"
    });

    let mut cmd = ctx.execute(bin);
    // If its VSCode (not VSCodium)
    if !VSCODIUM {
        // And we have configured use of a profile
        if let Some(profile) = ctx.config().vscode_profile() {
            // Add the profile argument
            cmd.arg("--profile").arg(profile);
        }
    }

    cmd.arg("--update-extensions").status_checked()
}

/// Make VSCodium a separate step because:
///
/// 1. Users could use both VSCode and VSCodium
/// 2. Just in case, VSCodium could have incompatible changes with VSCode
pub fn run_vscodium_extensions_update(ctx: &ExecutionContext) -> Result<()> {
    run_vscode_compatible::<true>(ctx)
}

pub fn run_vscode_extensions_update(ctx: &ExecutionContext) -> Result<()> {
    run_vscode_compatible::<false>(ctx)
}

pub fn run_pipx_update(ctx: &ExecutionContext) -> Result<()> {
    let pipx = require("pipx")?;
    print_separator("pipx");

    let mut command_args = vec!["upgrade-all", "--include-injected"];

    // pipx version 1.4.0 introduced a new command argument `pipx upgrade-all --quiet`
    // (see https://pipx.pypa.io/stable/docs/#pipx-upgrade-all)
    let version_str = Command::new(&pipx)
        .args(["--version"])
        .output_checked_utf8()
        .map(|s| s.stdout.trim().to_owned());
    let version = Version::parse(&version_str?);
    if matches!(version, Ok(version) if version >= Version::new(1, 4, 0)) {
        command_args.push("--quiet");
    }

    ctx.execute(pipx).args(command_args).status_checked()
}

pub fn run_pipxu_update(ctx: &ExecutionContext) -> Result<()> {
    let pipxu = require("pipxu")?;
    print_separator("pipxu");

    ctx.execute(pipxu).args(["upgrade", "--all"]).status_checked()
}

pub fn run_conda_update(ctx: &ExecutionContext) -> Result<()> {
    let conda = require("conda")?;

    let output = Command::new(&conda)
        .args(["config", "--show", "auto_activate_base"])
        .output_checked_utf8()?;
    debug!("Conda output: {}", output.stdout);
    if output.stdout.contains("False") {
        return Err(SkipStep("auto_activate_base is set to False".to_string()).into());
    }

    print_separator("Conda");

    // Update named environments, starting with the always-present "base"
    let base_env_name = "base".to_string();
    let addl_env_names = ctx.config().conda_env_names().into_iter().flatten();
    let env_names = once(&base_env_name).chain(addl_env_names);

    for env_name in env_names {
        let mut command = ctx.execute(&conda);
        command.args(["update", "--all", "-n", env_name]);
        if ctx.config().yes(Step::Conda) {
            command.arg("--yes");
        }
        command.status_checked()?;
    }

    // Update any environments given by path
    if let Some(env_paths) = ctx.config().conda_env_paths() {
        for env_path in env_paths.iter() {
            let mut command = ctx.execute(&conda);
            command.args(["update", "--all", "-p", env_path]);
            if ctx.config().yes(Step::Conda) {
                command.arg("--yes");
            }
            command.status_checked()?;
        }
    }

    // Cleanup (conda clean) is global (not tied to a particular environment)
    if ctx.config().cleanup() {
        let mut command = ctx.execute(conda);
        command.args(["clean", "--all"]);
        if ctx.config().yes(Step::Conda) {
            command.arg("--yes");
        }
        command.status_checked()?;
    }

    Ok(())
}

pub fn run_pixi_update(ctx: &ExecutionContext) -> Result<()> {
    let pixi = require("pixi")?;
    print_separator("Pixi");

    // Check if `pixi --help` mentions self-update, if yes, self-update must be enabled.
    // pixi self-update --help works regardless of whether the feature is enabled.
    let top_level_help_output = ctx.execute(&pixi).arg("--help").output_checked_utf8()?;

    if top_level_help_output.stdout.contains("self-update") {
        let self_update_help_output = ctx
            .execute(&pixi)
            .args(["self-update", "--help"])
            .output_checked_utf8()?;
        let mut cmd = ctx.execute(&pixi);
        cmd.arg("self-update");
        // check if help mentions --no-release-note to check if it is supported
        if self_update_help_output.stdout.contains("--no-release-note") && !ctx.config().show_pixi_release_notes() {
            cmd.arg("--no-release-note");
        }
        cmd.status_checked()?;
    }

    ctx.execute(&pixi).args(["global", "update"]).status_checked()
}

pub fn run_mamba_update(ctx: &ExecutionContext) -> Result<()> {
    let mamba = require("mamba")?;

    print_separator("Mamba");

    let mut command = ctx.execute(&mamba);
    command.args(["update", "--all", "-n", "base"]);
    if ctx.config().yes(Step::Mamba) {
        command.arg("--yes");
    }
    command.status_checked()?;

    if ctx.config().cleanup() {
        let mut command = ctx.execute(&mamba);
        command.args(["clean", "--all"]);
        if ctx.config().yes(Step::Mamba) {
            command.arg("--yes");
        }
        command.status_checked()?;
    }

    Ok(())
}

pub fn run_miktex_packages_update(ctx: &ExecutionContext) -> Result<()> {
    let miktex = require("miktex")?;
    print_separator("miktex");

    ctx.execute(miktex).args(["packages", "update"]).status_checked()
}

pub fn run_pip3_update(ctx: &ExecutionContext) -> Result<()> {
    let py = require("python").and_then(check_is_python_2_or_shim);
    let py3 = require("python3").and_then(check_is_python_2_or_shim);

    let python3 = match (py, py3) {
        // prefer `python` if it is available and is a valid Python 3.
        (Ok(py), _) => py,
        (Err(_), Ok(py3)) => py3,
        (Err(py_err), Err(py3_err)) => {
            return Err(SkipStep(format!("Skip due to following reasons: {py_err} {py3_err}")).into());
        }
    };

    Command::new(&python3)
        .args(["-m", "pip"])
        .output_checked_utf8()
        .map_err(|_| SkipStep("pip does not exist".to_string()))?;

    let check_extern_managed_script = "import sysconfig; from os import path; print('Y') if path.isfile(path.join(sysconfig.get_path('stdlib'), 'EXTERNALLY-MANAGED')) else print('N')";
    let output = Command::new(&python3)
        .args(["-c", check_extern_managed_script])
        .output_checked_utf8()?;
    let stdout = output.stdout.trim();
    let extern_managed = match stdout {
        "N" => false,
        "Y" => true,
        _ => unreachable!("unexpected output from `check_extern_managed_script`"),
    };

    let allow_break_sys_pkg = match Command::new(&python3)
        .args(["-m", "pip", "config", "get", "global.break-system-packages"])
        .output_checked_utf8()
    {
        Ok(output) => {
            let stdout = output.stdout.trim();
            stdout.parse::<bool>().wrap_err_with(|| {
                output_changed_message!(
                    "pip config get global.break-system-packages",
                    "unexpected output that is not `true` or `false`"
                )
            })?
        }
        // it can fail because this key may not be set
        //
        // ```sh
        // $ pip --version
        // pip 23.0.1 from /usr/lib/python3/dist-packages/pip (python 3.11)
        //
        // $ pip config get global.break-system-packages
        // ERROR: No such key - global.break-system-packages
        //
        // $ echo $?
        // 1
        // ```
        Err(_) => false,
    };

    debug!("pip3 externally managed: {} ", extern_managed);
    debug!("pip3 global.break-system-packages: {}", allow_break_sys_pkg);

    // Even though pip3 is externally managed, we should still update it if
    // `global.break-system-packages` is true.
    if extern_managed && !allow_break_sys_pkg {
        return Err(SkipStep(
            "Skip pip3 update as it is externally managed and global.break-system-packages is not true".to_string(),
        )
        .into());
    }

    print_separator("pip3");
    if env::var("VIRTUAL_ENV").is_ok() {
        print_warning("This step is skipped when running inside a virtual environment");
        return Err(SkipStep("Does not run inside a virtual environment".to_string()).into());
    }

    ctx.execute(&python3)
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
    ctx.execute(pip_review).arg("--auto").status_checked_with_codes(&[1])?;

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
    ctx.execute(pip_review)
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
    ctx.execute(pipupgrade)
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

    ctx.execute(stack).arg("upgrade").status_checked()
}

pub fn run_ghcup_update(ctx: &ExecutionContext) -> Result<()> {
    let ghcup = require("ghcup")?;
    print_separator("ghcup");

    ctx.execute(ghcup).arg("upgrade").status_checked()
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
        ctx.execute(&tlmgr)
    } else {
        let sudo = ctx.require_sudo()?;
        sudo.execute(ctx, &tlmgr)?
    };
    command.args(["update", "--self", "--all"]);

    command.status_checked()
}

pub fn run_chezmoi_update(ctx: &ExecutionContext) -> Result<()> {
    let chezmoi = require("chezmoi")?;
    HOME_DIR.join(".local/share/chezmoi").require()?;

    print_separator("chezmoi");

    ctx.execute(chezmoi).arg("update").status_checked()
}

pub fn run_myrepos_update(ctx: &ExecutionContext) -> Result<()> {
    let myrepos = require("mr")?;
    HOME_DIR.join(".mrconfig").require()?;

    print_separator("myrepos");

    ctx.execute(&myrepos)
        .arg("--directory")
        .arg(&*HOME_DIR)
        .arg("checkout")
        .status_checked()?;
    ctx.execute(&myrepos)
        .arg("--directory")
        .arg(&*HOME_DIR)
        .arg("update")
        .status_checked()
}

pub fn run_custom_command(name: &str, command: &str, ctx: &ExecutionContext) -> Result<()> {
    print_separator(name);
    let mut exec = ctx.execute(shell());
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
        .map_err(|e| (SkipStep(t!("Error getting the composer directory: {error}", error = e).to_string())))
        .map(|s| PathBuf::from(s.stdout.trim()))?
        .require()?;

    if !composer_home.is_descendant_of(&HOME_DIR) {
        return Err(SkipStep(
            t!(
                "Composer directory {composer_home} isn't a descendant of the user's home directory",
                composer_home = composer_home.display()
            )
            .to_string(),
        )
        .into());
    }

    print_separator(t!("Composer"));

    if ctx.config().composer_self_update() {
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                // If self-update fails without sudo then there's probably an update
                let has_update = match ctx.execute(&composer).arg("self-update").output()? {
                    ExecutorOutput::Wet(output) => !output.status.success(),
                    _ => false
                };

                if has_update {
                    let sudo = ctx.require_sudo()?;
                    sudo.execute(ctx, &composer)?
                       .arg("self-update")
                       .status_checked()?;
                }
            } else {
                ctx.execute(&composer).arg("self-update").status_checked()?;
            }
        }
    }

    let output = ctx.execute(&composer).args(["global", "update"]).output()?;
    if let ExecutorOutput::Wet(output) = output {
        let output: Utf8Output = output.try_into()?;
        print!("{}\n{}", output.stdout, output.stderr);
        if output.stdout.contains("valet") || output.stderr.contains("valet") {
            if let Some(valet) = which("valet") {
                ctx.execute(valet).arg("install").status_checked()?;
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
        .execute(&dotnet)
        .args(["tool", "list", "--global"])
        // dotnet will print a greeting message on its first run, from this question:
        // https://stackoverflow.com/q/70493706/14092446
        // Setting `DOTNET_NOLOGO` to `true` should disable it
        .env("DOTNET_NOLOGO", "true")
        .output_checked_utf8()
    {
        Ok(output) => output,
        Err(_) => {
            return Err(SkipStep(
                t!("Error running `dotnet tool list`. This is expected when a dotnet runtime is installed but no SDK.")
                    .to_string(),
            )
            .into());
        }
    };

    let mut in_header = true;
    let mut packages = output
        .stdout
        .lines()
        // Skip the header:
        //
        // Package Id      Version      Commands
        // -------------------------------------
        .skip_while(|line| {
            // The .NET SDK respects locale, so the header can be printed
            // in languages other than English. The separator should hopefully
            // always be at least 10 -'s long.
            if in_header && line.starts_with("----------") {
                in_header = false;
                true
            } else {
                in_header
            }
        })
        .filter(|line| !line.is_empty())
        .peekable();

    if packages.peek().is_none() {
        return Err(SkipStep(t!("No dotnet global tools installed").to_string()).into());
    }

    print_separator(".NET");

    for package in packages {
        let package_name = package.split_whitespace().next().unwrap();
        ctx.execute(&dotnet)
            .args(["tool", "update", package_name, "--global"])
            .status_checked()
            .with_context(|| format!("Failed to update .NET package {package_name:?}"))?;
    }

    Ok(())
}

pub fn run_powershell(ctx: &ExecutionContext) -> Result<()> {
    let powershell = ctx.require_powershell()?;

    print_separator(t!("Powershell Modules Update"));

    powershell.update_modules(ctx)
}

enum Hx {
    Helix(PathBuf),
    HxHexdump,
}

impl Hx {
    fn helix(self) -> Result<PathBuf> {
        match self {
            Hx::Helix(hx) => Ok(hx),
            Hx::HxHexdump => {
                Err(SkipStep("Command `hx` probably points to hx (hexdump alternative)".to_string()).into())
            }
        }
    }
}

fn get_hx(ctx: &ExecutionContext) -> Result<Hx> {
    let hx = require("hx")?;

    // Check if `hx --help` mentions "helix". Helix does, hx (hexdump alternative) doesn't.
    let output = ctx.execute(&hx).arg("--help").output_checked()?;

    if String::from_utf8(output.stdout)?.contains("helix") {
        debug!("Detected `hx` as Helix");
        Ok(Hx::Helix(hx))
    } else {
        debug!("Detected `hx` as hx (hexdump alternative)");
        Ok(Hx::HxHexdump)
    }
}

pub fn run_helix_grammars(ctx: &ExecutionContext) -> Result<()> {
    let helix = require("helix").or(get_hx(ctx)?.helix())?;

    print_separator("Helix");

    ctx.execute(&helix)
        .args(["--grammar", "fetch"])
        .status_checked()
        .with_context(|| "Failed to download helix grammars!")?;

    ctx.execute(&helix)
        .args(["--grammar", "build"])
        .status_checked()
        .with_context(|| "Failed to build helix grammars!")?;

    Ok(())
}

pub fn run_raco_update(ctx: &ExecutionContext) -> Result<()> {
    let raco = require("raco")?;

    print_separator(t!("Racket Package Manager"));

    ctx.execute(raco).args(["pkg", "update", "--all"]).status_checked()
}

pub fn bin_update(ctx: &ExecutionContext) -> Result<()> {
    let bin = require("bin")?;

    print_separator("Bin");
    ctx.execute(bin).arg("update").status_checked()
}

pub fn spicetify_upgrade(ctx: &ExecutionContext) -> Result<()> {
    // As of 04-07-2023 NixOS packages Spicetify with the `spicetify-cli` binary name
    let spicetify = require("spicetify").or(require("spicetify-cli"))?;

    print_separator("Spicetify");
    ctx.execute(spicetify).arg("upgrade").status_checked()
}

pub fn run_ghcli_extensions_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let gh = require("gh")?;
    let result = Command::new(&gh).args(["extensions", "list"]).output_checked_utf8();
    if result.is_err() {
        debug!("GH result {:?}", result);
        return Err(SkipStep(t!("GH failed").to_string()).into());
    }

    print_separator(t!("GitHub CLI Extensions"));
    ctx.execute(&gh)
        .args(["extension", "upgrade", "--all"])
        .status_checked()
}

pub fn update_julia_packages(ctx: &ExecutionContext) -> Result<()> {
    let julia = require("julia")?;

    print_separator(t!("Julia Packages"));

    let mut executor = ctx.execute(julia);

    executor.arg(if ctx.config().julia_use_startup_file() {
        "--startup-file=yes"
    } else {
        "--startup-file=no"
    });

    executor.args(["-e", "using Pkg; Pkg.update()"]).status_checked()
}

pub fn run_helm_repo_update(ctx: &ExecutionContext) -> Result<()> {
    let helm = require("helm")?;

    print_separator("Helm");

    let no_repo = "no repositories found";
    let mut success = true;
    let mut exec = ctx.execute(helm);
    if let Err(e) = exec.arg("repo").arg("update").status_checked() {
        error!("Updating repositories failed: {e}");
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
    ctx.execute(stew).args(["upgrade", "--all"]).status_checked()
}

pub fn run_bob(ctx: &ExecutionContext) -> Result<()> {
    let bob = require("bob")?;

    print_separator("Bob");

    ctx.execute(bob).args(["update", "--all"]).status_checked()
}

pub fn run_certbot(ctx: &ExecutionContext) -> Result<()> {
    let sudo = ctx.require_sudo()?;
    let certbot = require("certbot")?;

    print_separator("Certbot");

    sudo.execute(ctx, &certbot)?.arg("renew").status_checked()
}

/// Run `$ freshclam` to update ClamAV signature database
///
/// doc: https://docs.clamav.net/manual/Usage/SignatureManagement.html#freshclam
pub fn run_freshclam(ctx: &ExecutionContext) -> Result<()> {
    let freshclam = require("freshclam")?;
    print_separator(t!("Update ClamAV Database(FreshClam)"));

    let output = ctx.run_type().execute(&freshclam).output()?;
    let output = match output {
        ExecutorOutput::Wet(output) => output,
        ExecutorOutput::Dry => return Ok(()), // In a dry run, just exit after running without sudo
    };

    // Check if running without sudo was successful
    if output.status.success() {
        // Success, so write the output and exit
        std::io::stdout().lock().write_all(&output.stdout).unwrap();
        std::io::stderr().lock().write_all(&output.stderr).unwrap();
        return Ok(());
    }

    // Since running without sudo failed (hopefully due to permission errors), try running with sudo.
    debug!("`freshclam` (without sudo) resulted in error: {:?}", output);
    let sudo = ctx.require_sudo()?;

    match sudo.execute(ctx, freshclam).status_checked() {
        Ok(()) => Ok(()), // Success! The output of only the sudo'ed process is written.
        Err(err) => {
            // Error! We add onto the error the output of running without sudo for more information.
            Err(err.wrap_err(format!(
                "Running `freshclam` with sudo failed as well as running without sudo. Output without sudo: {output:?}"
            )))
        }
    }
}

/// Involve `pio upgrade` to update PlatformIO core.
pub fn run_platform_io(ctx: &ExecutionContext) -> Result<()> {
    // We use the full path because by default the binary is not in `PATH`:
    // https://github.com/topgrade-rs/topgrade/issues/754#issuecomment-2020537559
    #[cfg(unix)]
    fn bin_path() -> PathBuf {
        HOME_DIR.join(".platformio/penv/bin/pio")
    }
    #[cfg(windows)]
    fn bin_path() -> PathBuf {
        HOME_DIR.join(".platformio/penv/Scripts/pio.exe")
    }

    let bin_path = require(bin_path())?;

    print_separator("PlatformIO Core");

    ctx.execute(bin_path).arg("upgrade").status_checked()
}

/// Run `lensfun-update-data` to update lensfun database.
///
/// `sudo` will be used if `use_sudo` configuration entry is set to true.
pub fn run_lensfun_update_data(ctx: &ExecutionContext) -> Result<()> {
    const SEPARATOR: &str = "Lensfun's database update";
    let lensfun_update_data = require("lensfun-update-data")?;
    const EXIT_CODE_WHEN_NO_UPDATE: i32 = 1;

    if ctx.config().lensfun_use_sudo() {
        let sudo = ctx.require_sudo()?;
        print_separator(SEPARATOR);
        sudo.execute(ctx, &lensfun_update_data)?
            // `lensfun-update-data` returns 1 when there is no update available
            // which should be considered success
            .status_checked_with_codes(&[EXIT_CODE_WHEN_NO_UPDATE])
    } else {
        print_separator(SEPARATOR);
        ctx.execute(lensfun_update_data)
            .status_checked_with_codes(&[EXIT_CODE_WHEN_NO_UPDATE])
    }
}

pub fn run_poetry(ctx: &ExecutionContext) -> Result<()> {
    let poetry = require("poetry")?;

    #[cfg(unix)]
    fn get_interpreter(poetry: &PathBuf) -> Result<(PathBuf, Option<OsString>)> {
        // Parse the standard Unix shebang line: #!interpreter [optional-arg]
        // Spaces and tabs on either side of interpreter are ignored.

        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        static SHEBANG_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^#![ \t]*([^ \t\n]+)(?:[ \t]+([^\n]+)?)?").unwrap());

        let script = fs::read(poetry)?;
        if let Some(c) = SHEBANG_REGEX.captures(&script) {
            let interpreter = OsStr::from_bytes(&c[1]).into();
            let args = c.get(2).map(|args| OsStr::from_bytes(args.as_bytes()).into());
            return Ok((interpreter, args));
        }

        Err(eyre!("Could not find shebang"))
    }
    #[cfg(windows)]
    fn get_interpreter(poetry: &PathBuf) -> Result<(PathBuf, Option<OsString>)> {
        // Parse the shebang line from scripts using https://bitbucket.org/vinay.sajip/simple_launcher,
        // such as those created by pip. In contrast to Unix shebang lines, interpreter paths can
        // contain spaces, if they are double-quoted.

        use std::str;

        static SHEBANG_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^#![ \t]*(?:"([^"\n]+)"|([^" \t\n]+))(?:[ \t]+([^\n]+)?)?"#).unwrap());

        let data = fs::read(poetry)?;

        let pos = match data.windows(4).rposition(|b| b == b"PK\x05\x06") {
            Some(i) => i,
            None => return Err(eyre!("Not a ZIP archive")),
        };

        let cdr_size = match data.get(pos + 12..pos + 16) {
            Some(b) => u32::from_le_bytes(b.try_into().unwrap()) as usize,
            None => return Err(eyre!("Invalid CDR size")),
        };
        let cdr_offset = match data.get(pos + 16..pos + 20) {
            Some(b) => u32::from_le_bytes(b.try_into().unwrap()) as usize,
            None => return Err(eyre!("Invalid CDR offset")),
        };
        if pos < cdr_size + cdr_offset {
            return Err(eyre!("Invalid ZIP archive"));
        }
        let arc_pos = pos - cdr_size - cdr_offset;
        match data[..arc_pos].windows(2).rposition(|b| b == b"#!") {
            Some(l) => {
                let line = &data[l..arc_pos - 1];
                if let Some(c) = SHEBANG_REGEX.captures(line) {
                    let interpreter = c.get(1).or_else(|| c.get(2)).unwrap();
                    // shebang line should be valid utf8
                    let interpreter = str::from_utf8(interpreter.as_bytes())?.into();
                    let args = match c.get(3) {
                        Some(args) => Some(str::from_utf8(args.as_bytes())?.into()),
                        None => None,
                    };
                    Ok((interpreter, args))
                } else {
                    Err(eyre!("Invalid shebang line"))
                }
            }
            None => Err(eyre!("Could not find shebang")),
        }
    }

    if ctx.config().poetry_force_self_update() {
        debug!("forcing poetry self update");
    } else {
        let (interp, interp_args) = get_interpreter(&poetry)
            .map_err(|e| SkipStep(format!("Could not find interpreter for {}: {}", poetry.display(), e)))?;
        debug!("poetry interpreter: {:?}, args: {:?}", interp, interp_args);

        let check_official_install_script =
            "import sys; from os import path; print('Y') if path.isfile(path.join(sys.prefix, 'poetry_env')) else print('N')";
        let mut command = Command::new(&interp);
        if let Some(args) = interp_args {
            command.arg(args);
        }
        let output = command
            .args(["-c", check_official_install_script])
            .output_checked_utf8()?;
        let stdout = output.stdout.trim();
        let official_install = match stdout {
            "N" => false,
            "Y" => true,
            _ => unreachable!("unexpected output from `check_official_install_script`"),
        };

        debug!("poetry is official install: {}", official_install);

        if !official_install {
            return Err(SkipStep("Not installed with the official script".to_string()).into());
        }
    }

    print_separator("Poetry");
    ctx.execute(&poetry).args(["self", "update"]).status_checked()
}

pub fn run_uv(ctx: &ExecutionContext) -> Result<()> {
    let uv_exec = require("uv")?;
    print_separator("uv");

    // 1. Run `uv self update` if the `uv` binary is built with the `self-update`
    //    cargo feature enabled.
    //
    // To check if this feature is enabled or not, different version of `uv` need
    // different approaches, we need to know the version first and handle them
    // separately.
    let uv_version_output = ctx.execute(&uv_exec).arg("--version").output_checked_utf8()?;
    // Multiple possible output formats are possible according to uv source code
    //
    // https://github.com/astral-sh/uv/blob/6b7f60c1eaa840c2e933a0fb056ab46f99c991a5/crates/uv-cli/src/version.rs#L28-L42
    //
    // For example:
    //  "uv 0.5.11 (c4d0caaee 2024-12-19)\n"
    //  "uv 0.5.11+1 (xxxd0cee 2024-12-20)\n"
    //  "uv 0.6.14\n"

    let uv_version_output_stdout = uv_version_output.stdout;

    let version_str = {
        // Trim the starting "uv" and " " (whitespace)
        let start_trimmed = uv_version_output_stdout
            .trim_start_matches("uv")
            .trim_start_matches(' ');
        // Remove the tailing part " (c4d0caaee 2024-12-19)\n", if it's there
        match start_trimmed.find(' ') {
            None => start_trimmed.trim_end_matches('\n'), // Otherwise, just strip the newline
            Some(i) => &start_trimmed[..i],
        }

        // After trimming, it should be a string in 2 possible formats, both can be handled by `Version::parse()`
        //
        // 1. "0.5.11"
        // 2. "0.5.11+1"
    };
    let version =
        Version::parse(version_str).wrap_err_with(|| output_changed_message!("uv --version", "Invalid version"))?;

    if version < Version::new(0, 4, 25) {
        // For uv before version 0.4.25 (exclusive), the `self` sub-command only
        // exists under the `self-update` feature, we run `uv self --help` to check
        // the feature gate.
        let self_update_feature_enabled = ctx.execute(&uv_exec).args(["self", "--help"]).output_checked().is_ok();

        if self_update_feature_enabled {
            ctx.execute(&uv_exec).args(["self", "update"]).status_checked()?;
        }
    } else {
        // After 0.4.25 (inclusive), running `uv self` succeeds regardless of the
        // feature gate, so the above approach won't work.
        //
        // We run `uv self update` directly, and ignore an error if it outputs:
        //
        // "error: uv was installed through an external package manager, and self-update is not available. Please use your package manager to update uv.\n"
        //
        // or:
        //
        // "
        // error: Self-update is only available for uv binaries installed via the standalone installation scripts.
        //
        // If you installed uv with pip, brew, or another package manager, update uv with `pip install --upgrade`, `brew upgrade`, or similar.
        // "
        //
        // These two error messages can both occur, in different situations.

        const ERROR_MSGS: [&str; 2] = [
            "uv was installed through an external package manager, and self-update is not available. Please use your package manager to update uv.",
            "Self-update is only available for uv binaries installed via the standalone installation scripts.",
        ];

        let output = ctx
            .execute(&uv_exec)
            .args(["self", "update"])
            // `output()` captures the output so that users won't see it for now.
            .output()
            .expect("this should be ok regardless of this child process's exit code");
        let output = match output {
            ExecutorOutput::Wet(wet) => wet,
            ExecutorOutput::Dry => unreachable!("the whole function returns when we run `uv --version` under dry-run"),
        };
        let stderr = std::str::from_utf8(&output.stderr).expect("output should be UTF-8 encoded");

        if ERROR_MSGS.iter().any(|&n| stderr.contains(n)) {
            // Feature `self-update` is disabled, nothing to do.
        } else {
            // Feature is enabled, flush the captured output so that users know we did the self-update.

            std::io::stdout().write_all(&output.stdout)?;
            std::io::stderr().write_all(&output.stderr)?;

            // And, if self update failed, fail the step as well.
            if !output.status.success() {
                return Err(eyre!("uv self update failed"));
            }
        }
    };

    // 2. Update the installed tools
    ctx.execute(&uv_exec)
        .args(["tool", "upgrade", "--all"])
        .status_checked()?;

    if ctx.config().cleanup() {
        // 3. Prune cache
        ctx.execute(&uv_exec).args(["cache", "prune"]).status_checked()?;
    }

    Ok(())
}

/// Involve `zvm upgrade` to update ZVM
pub fn run_zvm(ctx: &ExecutionContext) -> Result<()> {
    let zvm = require("zvm")?;

    print_separator("ZVM");

    ctx.execute(zvm).arg("upgrade").status_checked()
}

pub fn run_bun(ctx: &ExecutionContext) -> Result<()> {
    let bun = require("bun")?;

    print_separator("Bun");

    ctx.execute(bun).arg("upgrade").status_checked()
}

pub fn run_zigup(ctx: &ExecutionContext) -> Result<()> {
    let zigup = require("zigup")?;
    let config = ctx.config();

    print_separator("zigup");

    let mut path_args = Vec::new();
    if let Some(path) = config.zigup_path_link() {
        path_args.push("--path-link".to_owned());
        path_args.push(shellexpand::tilde(path).into_owned());
    }
    if let Some(path) = config.zigup_install_dir() {
        path_args.push("--install-dir".to_owned());
        path_args.push(shellexpand::tilde(path).into_owned());
    }

    for zig_version in config.zigup_target_versions() {
        ctx.execute(&zigup)
            .args(&path_args)
            .arg("fetch")
            .arg(&zig_version)
            .status_checked()?;

        if config.zigup_cleanup() {
            ctx.execute(&zigup)
                .args(&path_args)
                .arg("keep")
                .arg(&zig_version)
                .status_checked()?;
        }
    }

    if config.zigup_cleanup() {
        ctx.execute(zigup).args(&path_args).arg("clean").status_checked()?;
    }

    Ok(())
}

pub fn run_jetbrains_toolbox(_ctx: &ExecutionContext) -> Result<()> {
    let installation = find_jetbrains_toolbox();
    match installation {
        Err(FindError::NotFound) => {
            // Skip
            Err(SkipStep(format!("{}", t!("No JetBrains Toolbox installation found"))).into())
        }
        Err(FindError::UnsupportedOS(os)) => {
            // Skip
            Err(SkipStep(format!("{}", t!("Unsupported operating system {os}", os = os))).into())
        }
        Err(e) => {
            // Unexpected error
            println!(
                "{}",
                t!("jetbrains-toolbox-updater encountered an unexpected error during finding:")
            );
            println!("{e:?}");
            Err(StepFailed.into())
        }
        Ok(installation) => {
            print_separator("JetBrains Toolbox");

            match update_jetbrains_toolbox(installation) {
                Err(e) => {
                    // Unexpected error
                    println!(
                        "{}",
                        t!("jetbrains-toolbox-updater encountered an unexpected error during updating:")
                    );
                    println!("{e:?}");
                    Err(StepFailed.into())
                }
                Ok(()) => Ok(()),
            }
        }
    }
}

fn run_jetbrains_ide_generic<const IS_JETBRAINS: bool>(ctx: &ExecutionContext, bin: PathBuf, name: &str) -> Result<()> {
    let prefix = if IS_JETBRAINS { "JetBrains " } else { "" };
    print_separator(format!("{prefix}{name} plugins"));

    // The `update` command is undocumented, but tested on all of the below.
    let output = ctx.execute(&bin).arg("update").output()?;
    let output = match output {
        ExecutorOutput::Dry => return Ok(()),
        ExecutorOutput::Wet(output) => output,
    };
    // Write the output which we swallowed in all cases
    std::io::stdout().lock().write_all(&output.stdout).unwrap();
    std::io::stderr().lock().write_all(&output.stderr).unwrap();

    let stdout = String::from_utf8(output.stdout.clone()).wrap_err("Expected valid UTF-8 output")?;

    // "Only one instance of RustRover can be run at a time."
    if stdout.contains("Only one instance of ") && stdout.contains(" can be run at a time.") {
        // It's always paired with status code 1
        let status_code = output
            .status
            .code()
            .ok_or_eyre("Failed to get status code; was killed with signal")?;
        if status_code != 1 {
            return Err(eyre!("Expected status code 1 ('Only one instance of <IDE> can be run at a time.'), but found status code {}. Output: {output:?}", status_code));
        }
        // Don't crash, but don't be silent either
        warn!("{name} is already running, can't update it now.");
        Err(SkipStep(format!("{name} is already running, can't update it now.")).into())
    } else if !output.status.success() {
        // Unknown failure
        Err(eyre!("Running `{bin:?} update` failed. Output: {output:?}"))
    } else {
        // Success. Output was already written above
        Ok(())
    }
}

fn run_jetbrains_ide(ctx: &ExecutionContext, bin: PathBuf, name: &str) -> Result<()> {
    run_jetbrains_ide_generic::<true>(ctx, bin, name)
}

pub fn run_android_studio(ctx: &ExecutionContext) -> Result<()> {
    // We don't use `run_jetbrains_ide` here because that would print "JetBrains Android Studio",
    //  which is incorrect as Android Studio is made by Google. Just "Android Studio" is fine.
    run_jetbrains_ide_generic::<false>(
        ctx,
        require_one([
            "studio",
            "android-studio",
            "android-studio-beta",
            "android-studio-canary",
        ])?,
        "Android Studio",
    )
}

pub fn run_jetbrains_aqua(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(ctx, get_aqua(ctx)?.jetbrains_aqua()?, "Aqua")
}

pub fn run_jetbrains_clion(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(ctx, require_one(["clion", "clion-eap"])?, "CLion")
}

pub fn run_jetbrains_datagrip(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(ctx, require_one(["datagrip", "datagrip-eap"])?, "DataGrip")
}

pub fn run_jetbrains_dataspell(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(ctx, require_one(["dataspell", "dataspell-eap"])?, "DataSpell")
}

pub fn run_jetbrains_gateway(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(
        ctx,
        require_one(["gateway", "jetbrains-gateway", "jetbrains-gateway-eap"])?,
        "Gateway",
    )
}

pub fn run_jetbrains_goland(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(ctx, require_one(["goland", "goland-eap"])?, "Goland")
}

pub fn run_jetbrains_idea(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(
        ctx,
        require_one([
            "idea",
            "intellij-idea-ultimate-edition",
            "intellij-idea-community-edition",
        ])?,
        "IntelliJ IDEA",
    )
}

pub fn run_jetbrains_mps(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(ctx, require_one(["mps", "jetbrains-mps"])?, "MPS")
}

pub fn run_jetbrains_phpstorm(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(ctx, require("phpstorm")?, "PhpStorm")
}

pub fn run_jetbrains_pycharm(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(
        ctx,
        require_one(["pycharm", "pycharm-professional", "pycharm-eap"])?,
        "PyCharm",
    )
}

pub fn run_jetbrains_rider(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(ctx, require_one(["rider", "rider-eap"])?, "Rider")
}

pub fn run_jetbrains_rubymine(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(
        ctx,
        require_one(["rubymine", "jetbrains-rubymine", "rubymine-eap"])?,
        "RubyMine",
    )
}

pub fn run_jetbrains_rustrover(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(ctx, require_one(["rustrover", "rustrover-eap"])?, "RustRover")
}

pub fn run_jetbrains_webstorm(ctx: &ExecutionContext) -> Result<()> {
    run_jetbrains_ide(ctx, require_one(["webstorm", "webstorm-eap"])?, "WebStorm")
}

pub fn run_yazi(ctx: &ExecutionContext) -> Result<()> {
    let ya = require("ya")?;

    print_separator("Yazi packages");

    ctx.execute(ya).args(["pkg", "upgrade"]).status_checked()
}
