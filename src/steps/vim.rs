use crate::error::{SkipStep, TopgradeError};
use anyhow::Result;

use crate::executor::{CommandExt, Executor, ExecutorOutput, RunType};
use crate::terminal::print_separator;
use crate::{
    execution_context::ExecutionContext,
    utils::{require, PathExt},
};
use directories::BaseDirs;
use log::debug;
use std::path::PathBuf;
use std::{
    io::{self, Write},
    process::Command,
};

const UPGRADE_VIM: &str = include_str!("upgrade.vim");

pub fn vimrc(base_dirs: &BaseDirs) -> Result<PathBuf> {
    base_dirs
        .home_dir()
        .join(".vimrc")
        .require()
        .or_else(|_| base_dirs.home_dir().join(".vim/vimrc").require())
}

fn nvimrc(base_dirs: &BaseDirs) -> Result<PathBuf> {
    #[cfg(unix)]
        let base_dir =
        // Bypass directories crate as nvim doesn't use the macOS-specific directories.
        std::env::var_os("XDG_CONFIG_HOME").map_or_else(|| base_dirs.home_dir().join(".config"), PathBuf::from);

    #[cfg(windows)]
    let base_dir = base_dirs.cache_dir();

    base_dir
        .join("nvim/init.vim")
        .require()
        .or_else(|_| base_dir.join("nvim/init.lua").require())
}

fn upgrade_script() -> Result<tempfile::NamedTempFile> {
    let mut tempfile = tempfile::NamedTempFile::new()?;
    tempfile.write_all(UPGRADE_VIM.replace('\r', "").as_bytes())?;
    debug!("Wrote vim script to {:?}", tempfile.path());
    Ok(tempfile)
}

fn upgrade(command: &mut Executor, ctx: &ExecutionContext) -> Result<()> {
    if ctx.config().force_vim_plug_update() {
        command.env("TOPGRADE_FORCE_PLUGUPDATE", "true");
    }

    let output = command.output()?;

    if let ExecutorOutput::Wet(output) = output {
        let status = output.status;

        if !status.success() || ctx.config().verbose() {
            io::stdout().write(&output.stdout).ok();
            io::stderr().write(&output.stderr).ok();
        }

        if !status.success() {
            return Err(TopgradeError::ProcessFailed(status).into());
        } else {
            println!("Plugins upgraded")
        }
    }

    Ok(())
}

pub fn upgrade_ultimate_vimrc(ctx: &ExecutionContext) -> Result<()> {
    let config_dir = ctx.base_dirs().home_dir().join(".vim_runtime").require()?;
    let git = require("git")?;
    let python = require("python3")?;
    let update_plugins = config_dir.join("update_plugins.py").require()?;

    print_separator("The Ultimate vimrc");

    ctx.run_type()
        .execute(&git)
        .current_dir(&config_dir)
        .args(["reset", "--hard"])
        .check_run()?;
    ctx.run_type()
        .execute(&git)
        .current_dir(&config_dir)
        .args(["clean", "-d", "--force"])
        .check_run()?;
    ctx.run_type()
        .execute(&git)
        .current_dir(&config_dir)
        .args(["pull", "--rebase"])
        .check_run()?;
    ctx.run_type()
        .execute(python)
        .current_dir(config_dir)
        .arg(update_plugins)
        .check_run()?;

    Ok(())
}

pub fn upgrade_vim(base_dirs: &BaseDirs, ctx: &ExecutionContext) -> Result<()> {
    let vim = require("vim")?;

    let output = Command::new(&vim).arg("--version").check_output()?;
    if !output.starts_with("VIM") {
        return Err(SkipStep(String::from("vim binary might be actually nvim")).into());
    }

    let vimrc = vimrc(base_dirs)?;

    print_separator("Vim");
    upgrade(
        ctx.run_type()
            .execute(&vim)
            .args(["-u"])
            .arg(vimrc)
            .args(["-U", "NONE", "-V1", "-nNesS"])
            .arg(upgrade_script()?.path()),
        ctx,
    )
}

pub fn upgrade_neovim(base_dirs: &BaseDirs, ctx: &ExecutionContext) -> Result<()> {
    let nvim = require("nvim")?;
    let nvimrc = nvimrc(base_dirs)?;

    print_separator("Neovim");
    upgrade(
        ctx.run_type()
            .execute(nvim)
            .args(["-u"])
            .arg(nvimrc)
            .args(["--headless", "-V1", "-nS"])
            .arg(upgrade_script()?.path()),
        ctx,
    )
}

pub fn run_voom(_base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let voom = require("voom")?;

    print_separator("voom");

    run_type.execute(voom).arg("update").check_run()
}
