use std::fmt::Display;
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::Command;

use crate::utils::{require_option, REQUIRE_SUDO};
use crate::HOME_DIR;
use color_eyre::eyre::Result;
#[cfg(target_os = "linux")]
use nix::unistd::Uid;
use semver::Version;
use tracing::debug;

use crate::command::CommandExt;
use crate::terminal::print_separator;
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
        Version::parse(&version_str?).map_err(|err| err.into())
    }

    fn upgrade(&self, ctx: &ExecutionContext, use_sudo: bool) -> Result<()> {
        let args = ["update", self.global_location_arg()];
        if use_sudo {
            let sudo = require_option(ctx.sudo().clone(), REQUIRE_SUDO.to_string())?;
            ctx.run_type()
                .execute(sudo)
                .arg(&self.command)
                .args(args)
                .status_checked()?;
        } else {
            ctx.run_type().execute(&self.command).args(args).status_checked()?;
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
            let sudo = require_option(ctx.sudo().clone(), REQUIRE_SUDO.to_string())?;
            ctx.run_type()
                .execute(sudo)
                .arg(self.yarn.as_ref().unwrap_or(&self.command))
                .args(args)
                .status_checked()?;
        } else {
            ctx.run_type().execute(&self.command).args(args).status_checked()?;
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

    print_separator("Node Package Manager");

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

    print_separator("Performant Node Package Manager");

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

    print_separator("Yarn Package Manager");

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
    let deno = require("deno")?;
    let deno_dir = HOME_DIR.join(".deno");

    if !deno.canonicalize()?.is_descendant_of(&deno_dir) {
        let skip_reason = SkipStep("Deno installed outside of .deno directory".to_string());
        return Err(skip_reason.into());
    }

    print_separator("Deno");
    ctx.run_type().execute(&deno).arg("upgrade").status_checked()
}
