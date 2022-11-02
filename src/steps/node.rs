#![allow(unused_imports)]

#[cfg(unix)]
use std::os::unix::prelude::MetadataExt;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;
use directories::BaseDirs;
use log::debug;
#[cfg(unix)]
use nix::unistd::Uid;
use semver::Version;

use crate::executor::{CommandExt, RunType};
use crate::terminal::print_separator;
use crate::utils::{require, PathExt};
use crate::{error::SkipStep, execution_context::ExecutionContext};

#[allow(clippy::upper_case_acronyms)]
struct NPM {
    command: PathBuf,
}

impl NPM {
    fn new(command: PathBuf) -> Self {
        Self { command }
    }

    #[cfg(target_os = "linux")]
    fn root(&self) -> Result<PathBuf> {
        let version = self.version()?;
        let args = if version < Version::new(8, 11, 0) {
            ["root", "-g"]
        } else {
            ["root", "--location=global"]
        };
        Command::new(&self.command)
            .args(args)
            .check_output()
            .map(|s| PathBuf::from(s.trim()))
    }

    fn version(&self) -> Result<Version> {
        let version_str = Command::new(&self.command)
            .args(["--version"])
            .check_output()
            .map(|s| s.trim().to_owned());
        Version::parse(&version_str?).map_err(|err| err.into())
    }

    fn upgrade(&self, run_type: RunType, use_sudo: bool) -> Result<()> {
        print_separator("Node Package Manager");
        let version = self.version()?;
        let args = if version < Version::new(8, 11, 0) {
            ["update", "-g"]
        } else {
            ["update", "--location=global"]
        };
        if use_sudo {
            run_type.execute("sudo").args(args).check_run()?;
        } else {
            run_type.execute(&self.command).args(args).check_run()?;
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn should_use_sudo(&self) -> Result<bool> {
        let npm_root = self.root()?;
        if !npm_root.exists() {
            return Err(SkipStep(format!("NPM root at {} doesn't exist", npm_root.display(),)).into());
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
        let version = Command::new(&self.command).args(["--version"]).check_output();

        matches!(version, Ok(ver) if ver.starts_with('1') || ver.starts_with('0'))
    }

    #[cfg(target_os = "linux")]
    fn root(&self) -> Result<PathBuf> {
        let args = ["global", "dir"];
        Command::new(&self.command)
            .args(args)
            .check_output()
            .map(|s| PathBuf::from(s.trim()))
    }

    fn upgrade(&self, run_type: RunType, use_sudo: bool) -> Result<()> {
        print_separator("Yarn Package Manager");
        let args = ["global", "upgrade"];

        if use_sudo {
            run_type
                .execute("sudo")
                .arg(self.yarn.as_ref().unwrap_or(&self.command))
                .args(args)
                .check_run()?;
        } else {
            run_type.execute(&self.command).args(args).check_run()?;
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn should_use_sudo(&self) -> Result<bool> {
        let yarn_root = self.root()?;
        if !yarn_root.exists() {
            return Err(SkipStep(format!("NPM root at {} doesn't exist", yarn_root.display(),)).into());
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
    let npm = require("pnpm").or_else(|_| require("npm")).map(NPM::new)?;

    #[cfg(target_os = "linux")]
    {
        npm.upgrade(ctx.run_type(), should_use_sudo(&npm, ctx)?)
    }

    #[cfg(not(target_os = "linux"))]
    {
        npm.upgrade(ctx.run_type(), false)
    }
}

pub fn run_yarn_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let yarn = require("yarn").map(Yarn::new)?;

    if !yarn.has_global_subcmd() {
        debug!("Yarn is 2.x or above, skipping global upgrade");
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        yarn.upgrade(ctx.run_type(), should_use_sudo_yarn(&yarn, ctx)?)
    }

    #[cfg(not(target_os = "linux"))]
    {
        yarn.upgrade(ctx.run_type(), false)
    }
}

pub fn deno_upgrade(ctx: &ExecutionContext) -> Result<()> {
    let deno = require("deno")?;
    let deno_dir = ctx.base_dirs().home_dir().join(".deno");

    if !deno.canonicalize()?.is_descendant_of(&deno_dir) {
        let skip_reason = SkipStep("Deno installed outside of .deno directory".to_string());
        return Err(skip_reason.into());
    }

    print_separator("Deno");
    ctx.run_type().execute(&deno).arg("upgrade").check_run()
}
