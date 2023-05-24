use crate::command::CommandExt;
use crate::error::{SkipStep, TopgradeError};
use crate::HOME_DIR;
use color_eyre::eyre::Result;
use etcetera::base_strategy::BaseStrategy;

use crate::executor::{Executor, ExecutorOutput};
use crate::terminal::print_separator;
use crate::{
    execution_context::ExecutionContext,
    utils::{require, PathExt},
};
use std::path::PathBuf;
use std::{
    io::{self, Write},
    process::Command,
};
use tracing::debug;

const UPGRADE_VIM: &str = include_str!("upgrade.vim");

pub fn vimrc() -> Result<PathBuf> {
    HOME_DIR
        .join(".vimrc")
        .require()
        .or_else(|_| HOME_DIR.join(".vim/vimrc").require())
}

fn nvimrc() -> Result<PathBuf> {
    #[cfg(unix)]
    let base_dir = crate::XDG_DIRS.config_dir();

    #[cfg(windows)]
    let base_dir = crate::WINDOWS_DIRS.cache_dir();

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
            return Err(TopgradeError::ProcessFailed(command.get_program(), status).into());
        } else {
            println!("Plugins upgraded")
        }
    }

    Ok(())
}

pub fn upgrade_ultimate_vimrc(ctx: &ExecutionContext) -> Result<()> {
    let config_dir = HOME_DIR.join(".vim_runtime").require()?;
    let git = require("git")?;
    let python = require("python3")?;
    let update_plugins = config_dir.join("update_plugins.py").require()?;

    print_separator("The Ultimate vimrc");

    ctx.run_type()
        .execute(&git)
        .current_dir(&config_dir)
        .args(["reset", "--hard"])
        .status_checked()?;
    ctx.run_type()
        .execute(&git)
        .current_dir(&config_dir)
        .args(["clean", "-d", "--force"])
        .status_checked()?;
    ctx.run_type()
        .execute(&git)
        .current_dir(&config_dir)
        .args(["pull", "--rebase"])
        .status_checked()?;
    ctx.run_type()
        .execute(python)
        .current_dir(config_dir)
        .arg(update_plugins)
        .status_checked()?;

    Ok(())
}

pub fn upgrade_vim(ctx: &ExecutionContext) -> Result<()> {
    let vim = require("vim")?;

    let output = Command::new(&vim).arg("--version").output_checked_utf8()?;
    if !output.stdout.starts_with("VIM") {
        return Err(SkipStep(String::from("vim binary might be actually nvim")).into());
    }

    let vimrc = vimrc()?;

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

pub fn upgrade_neovim(ctx: &ExecutionContext) -> Result<()> {
    let nvim = require("nvim")?;
    let nvimrc = nvimrc()?;

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

pub fn run_voom(ctx: &ExecutionContext) -> Result<()> {
    let voom = require("voom")?;

    print_separator("voom");

    ctx.run_type().execute(voom).arg("update").status_checked()
}
