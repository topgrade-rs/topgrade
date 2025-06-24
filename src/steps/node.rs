use std::fmt::Display;
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::Command;

use crate::utils::{get_require_sudo_string, require_option};
use crate::HOME_DIR;
use color_eyre::eyre::Result;
#[cfg(target_os = "linux")]
use nix::unistd::Uid;
use rust_i18n::t;
use semver::Version;
use tracing::debug;

use crate::command::CommandExt;
use crate::terminal::{print_info, print_separator};
use crate::utils::{require, PathExt};
use crate::{error::SkipStep, execution_context::ExecutionContext};

enum NPMVariant {
    Npm,
    Pnpm,
}

impl NPMVariant {
    const fn short_name(&self) -> &str {
        match self {
            NPMVariant::Npm => "npm",
            NPMVariant::Pnpm => "pnpm",
        }
    }

    const fn is_npm(&self) -> bool {
        matches!(self, NPMVariant::Npm)
    }
}

impl Display for NPMVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.short_name())
    }
}

#[allow(clippy::upper_case_acronyms)]
struct NPM {
    command: PathBuf,
    variant: NPMVariant,
}

impl NPM {
    fn new(command: PathBuf, variant: NPMVariant) -> Self {
        Self { command, variant }
    }

    /// Is the “NPM” version larger than 8.11.0?
    fn is_npm_8(&self) -> bool {
        let v = self.version();

        self.variant.is_npm() && matches!(v, Ok(v) if v >= Version::new(8, 11, 0))
    }

    /// Get the most suitable “global location” argument
    /// of this NPM instance.
    ///
    /// If the “NPM” version is larger than 8.11.0, we use
    /// `--location=global`; otherwise, use `-g`.
    fn global_location_arg(&self) -> &str {
        if self.is_npm_8() {
            "--location=global"
        } else {
            "-g"
        }
    }

    #[cfg(target_os = "linux")]
    fn root(&self) -> Result<PathBuf> {
        let args = ["root", self.global_location_arg()];
        Command::new(&self.command)
            .args(args)
            .output_checked_utf8()
            .map(|s| PathBuf::from(s.stdout.trim()))
    }

    fn version(&self) -> Result<Version> {
        let version_str = Command::new(&self.command)
            .args(["--version"])
            .output_checked_utf8()
            .map(|s| s.stdout.trim().to_owned());
        Version::parse(&version_str?).map_err(std::convert::Into::into)
    }

    fn upgrade(&self, ctx: &ExecutionContext, use_sudo: bool) -> Result<()> {
        let args = ["update", self.global_location_arg()];
        if use_sudo {
            let sudo = require_option(ctx.sudo().clone(), get_require_sudo_string())?;
            ctx.execute(sudo).arg(&self.command).args(args).status_checked()?;
        } else {
            ctx.execute(&self.command).args(args).status_checked()?;
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn should_use_sudo(&self) -> Result<bool> {
        let npm_root = self.root()?;
        if !npm_root.exists() {
            return Err(SkipStep(format!("{} root at {} doesn't exist", self.variant, npm_root.display())).into());
        }

        let metadata = std::fs::metadata(&npm_root)?;
        let uid = Uid::effective();

        Ok(metadata.uid() != uid.as_raw() && metadata.uid() == 0)
    }
}

struct Yarn {
    command: PathBuf,
    yarn: Option<PathBuf>,
}

impl Yarn {
    fn new(command: PathBuf) -> Self {
        Self {
            command,
            yarn: require("yarn").ok(),
        }
    }

    fn has_global_subcmd(&self) -> bool {
        // Get the version of Yarn. After Yarn 2.x (berry),
        // “yarn global” has been replaced with “yarn dlx”.
        //
        // As “yarn dlx” don't need to “upgrade”, we
        // ignore the whole task if Yarn is 2.x or above.
        let version = Command::new(&self.command).args(["--version"]).output_checked_utf8();

        matches!(version, Ok(ver) if ver.stdout.starts_with('1') || ver.stdout.starts_with('0'))
    }

    #[cfg(target_os = "linux")]
    fn root(&self) -> Result<PathBuf> {
        let args = ["global", "dir"];
        Command::new(&self.command)
            .args(args)
            .output_checked_utf8()
            .map(|s| PathBuf::from(s.stdout.trim()))
    }

    fn upgrade(&self, ctx: &ExecutionContext, use_sudo: bool) -> Result<()> {
        let args = ["global", "upgrade"];

        if use_sudo {
            let sudo = require_option(ctx.sudo().clone(), get_require_sudo_string())?;
            ctx.execute(sudo)
                .arg(self.yarn.as_ref().unwrap_or(&self.command))
                .args(args)
                .status_checked()?;
        } else {
            ctx.execute(&self.command).args(args).status_checked()?;
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn should_use_sudo(&self) -> Result<bool> {
        let yarn_root = self.root()?;
        if !yarn_root.exists() {
            return Err(SkipStep(format!("Yarn root at {} doesn't exist", yarn_root.display(),)).into());
        }

        let metadata = std::fs::metadata(&yarn_root)?;
        let uid = Uid::effective();

        Ok(metadata.uid() != uid.as_raw() && metadata.uid() == 0)
    }
}

struct Deno {
    command: PathBuf,
}

impl Deno {
    fn new(command: PathBuf) -> Self {
        Self { command }
    }

    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        let mut args = vec![];

        let version = ctx.config().deno_version();
        if let Some(version) = version {
            let bin_version = self.version()?;

            if bin_version >= Version::new(2, 0, 0) {
                args.push(version);
            } else if bin_version >= Version::new(1, 6, 0) {
                match version {
                    "stable" => { /* do nothing, as stable is the default channel to upgrade */ }
                    "rc" => {
                        return Err(SkipStep(
                            "Deno (1.6.0-2.0.0) cannot be upgraded to a release candidate".to_string(),
                        )
                        .into());
                    }
                    "canary" => args.push("--canary"),
                    _ => {
                        if Version::parse(version).is_err() {
                            return Err(SkipStep("Invalid Deno version".to_string()).into());
                        }

                        args.push("--version");
                        args.push(version);
                    }
                }
            } else if bin_version >= Version::new(1, 0, 0) {
                match version {
                    "stable" | "rc" | "canary" => {
                        // Prior to v1.6.0, `deno upgrade` is not able fetch the latest tag version.
                        return Err(
                            SkipStep("Deno (1.0.0-1.6.0) cannot be upgraded to a named channel".to_string()).into(),
                        );
                    }
                    _ => {
                        if Version::parse(version).is_err() {
                            return Err(SkipStep("Invalid Deno version".to_string()).into());
                        }

                        args.push("--version");
                        args.push(version);
                    }
                }
            } else {
                // v0.x cannot be upgraded with `deno upgrade` to v1.x or v2.x
                // nor can be upgraded to a specific version.
                return Err(SkipStep("Unsupported Deno version".to_string()).into());
            }
        }

        ctx.execute(&self.command).arg("upgrade").args(args).status_checked()?;
        Ok(())
    }

    /// Get the version of Deno.
    ///
    /// This function will return the version of Deno installed on the system.
    /// The version is parsed from the output of `deno -V`.
    ///
    /// ```sh
    /// deno -V # deno 1.6.0
    /// ```
    fn version(&self) -> Result<Version> {
        let version_str = Command::new(&self.command)
            .args(["-V"])
            .output_checked_utf8()
            .map(|s| s.stdout.trim().to_owned().split_off(5)); // remove "deno " prefix
        Version::parse(&version_str?).map_err(std::convert::Into::into)
    }
}

#[cfg(target_os = "linux")]
fn should_use_sudo(npm: &NPM, ctx: &ExecutionContext) -> Result<bool> {
    if npm.should_use_sudo()? {
        if ctx.config().npm_use_sudo() {
            Ok(true)
        } else {
            Err(SkipStep("NPM root is owned by another user which is not the current user. Set use_sudo = true under the NPM section in your configuration to run NPM as sudo".to_string())
                .into())
        }
    } else {
        Ok(false)
    }
}

#[cfg(target_os = "linux")]
fn should_use_sudo_yarn(yarn: &Yarn, ctx: &ExecutionContext) -> Result<bool> {
    if yarn.should_use_sudo()? {
        if ctx.config().yarn_use_sudo() {
            Ok(true)
        } else {
            Err(SkipStep("NPM root is owned by another user which is not the current user. Set use_sudo = true under the NPM section in your configuration to run NPM as sudo".to_string())
                .into())
        }
    } else {
        Ok(false)
    }
}

pub fn run_npm_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let npm = require("npm").map(|b| NPM::new(b, NPMVariant::Npm))?;

    print_separator(t!("Node Package Manager"));

    #[cfg(target_os = "linux")]
    {
        npm.upgrade(ctx, should_use_sudo(&npm, ctx)?)
    }

    #[cfg(not(target_os = "linux"))]
    {
        npm.upgrade(ctx, false)
    }
}

pub fn run_pnpm_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let pnpm = require("pnpm").map(|b| NPM::new(b, NPMVariant::Pnpm))?;

    print_separator(t!("Performant Node Package Manager"));

    #[cfg(target_os = "linux")]
    {
        pnpm.upgrade(ctx, should_use_sudo(&pnpm, ctx)?)
    }

    #[cfg(not(target_os = "linux"))]
    {
        pnpm.upgrade(ctx, false)
    }
}

pub fn run_yarn_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let yarn = require("yarn").map(Yarn::new)?;

    if !yarn.has_global_subcmd() {
        debug!("Yarn is 2.x or above, skipping global upgrade");
        return Ok(());
    }

    print_separator(t!("Yarn Package Manager"));

    #[cfg(target_os = "linux")]
    {
        yarn.upgrade(ctx, should_use_sudo_yarn(&yarn, ctx)?)
    }

    #[cfg(not(target_os = "linux"))]
    {
        yarn.upgrade(ctx, false)
    }
}

pub fn deno_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let deno = require("deno").map(Deno::new)?;
    let deno_dir = HOME_DIR.join(".deno");

    if !deno.command.canonicalize()?.is_descendant_of(&deno_dir) {
        let skip_reason = SkipStep(t!("Deno installed outside of .deno directory").to_string());
        return Err(skip_reason.into());
    }

    print_separator("Deno");
    deno.upgrade(ctx)
}

/// There is no `volta upgrade` command, so we need to upgrade each package
pub fn run_volta_packages_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let volta = require("volta")?;

    print_separator("Volta");

    if ctx.run_type().dry() {
        print_info(t!("Updating Volta packages..."));
        return Ok(());
    }

    let list_output = ctx
        .execute(&volta)
        .args(["list", "--format=plain"])
        .output_checked_utf8()?
        .stdout;

    let installed_packages: Vec<&str> = list_output
        .lines()
        .filter_map(|line| {
            // format is 'kind package@version ...'
            let mut parts = line.split_whitespace();
            parts.next();
            let package_part = parts.next()?;
            let version_index = package_part.rfind('@').unwrap_or(package_part.len());
            Some(package_part[..version_index].trim())
        })
        .collect();

    if installed_packages.is_empty() {
        print_info(t!("No packages installed with Volta"));
        return Ok(());
    }

    for package in &installed_packages {
        ctx.execute(&volta).args(["install", package]).status_checked()?;
    }

    Ok(())
}
