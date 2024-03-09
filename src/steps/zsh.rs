use std::env;
use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::Result;
use tracing::debug;
use walkdir::WalkDir;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::git::RepoStep;
use crate::terminal::print_separator;
use crate::utils::{require, PathExt};
use crate::HOME_DIR;
use crate::XDG_DIRS;
use etcetera::base_strategy::BaseStrategy;

pub fn run_zr(ctx: &ExecutionContext) -> Result<()> {
    let zsh = require("zsh")?;

    require("zr")?;

    print_separator("zr");

    let cmd = format!("source {} && zr --update", zshrc().display());
    ctx.run_type()
        .execute(zsh)
        .args(["-l", "-c", cmd.as_str()])
        .status_checked()
}

fn zdotdir() -> PathBuf {
    env::var("ZDOTDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| HOME_DIR.clone())
}

pub fn zshrc() -> PathBuf {
    zdotdir().join(".zshrc")
}

pub fn run_antidote(ctx: &ExecutionContext) -> Result<()> {
    let zsh = require("zsh")?;
    let mut antidote = zdotdir().join(".antidote").require()?;
    antidote.push("antidote.zsh");

    print_separator("antidote");

    ctx.run_type()
        .execute(zsh)
        .arg("-c")
        .arg(format!("source {} && antidote update", antidote.display()))
        .status_checked()
}

pub fn run_antibody(ctx: &ExecutionContext) -> Result<()> {
    require("zsh")?;
    let antibody = require("antibody")?;

    print_separator("antibody");

    ctx.run_type().execute(antibody).arg("update").status_checked()
}

pub fn run_antigen(ctx: &ExecutionContext) -> Result<()> {
    let zsh = require("zsh")?;
    let zshrc = zshrc().require()?;
    env::var("ADOTDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| HOME_DIR.join("antigen.zsh"))
        .require()?;

    print_separator("antigen");

    let cmd = format!("source {} && (antigen selfupdate ; antigen update)", zshrc.display());
    ctx.run_type()
        .execute(zsh)
        .args(["-l", "-c", cmd.as_str()])
        .status_checked()
}

pub fn run_zgenom(ctx: &ExecutionContext) -> Result<()> {
    let zsh = require("zsh")?;
    let zshrc = zshrc().require()?;
    env::var("ZGEN_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| HOME_DIR.join(".zgenom"))
        .require()?;

    print_separator("zgenom");

    let cmd = format!("source {} && zgenom selfupdate && zgenom update", zshrc.display());
    ctx.run_type()
        .execute(zsh)
        .args(["-l", "-c", cmd.as_str()])
        .status_checked()
}

pub fn run_zplug(ctx: &ExecutionContext) -> Result<()> {
    let zsh = require("zsh")?;
    zshrc().require()?;

    env::var("ZPLUG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| HOME_DIR.join(".zplug"))
        .require()?;

    print_separator("zplug");

    ctx.run_type()
        .execute(zsh)
        .args(["-i", "-c", "zplug update"])
        .status_checked()
}

pub fn run_zinit(ctx: &ExecutionContext) -> Result<()> {
    let zsh = require("zsh")?;
    let zshrc = zshrc().require()?;

    env::var("ZINIT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| XDG_DIRS.data_dir().join("zinit"))
        .require()?;

    print_separator("zinit");

    let cmd = format!("source {} && zinit self-update && zinit update --all", zshrc.display());
    ctx.run_type()
        .execute(zsh)
        .args(["-i", "-c", cmd.as_str()])
        .status_checked()
}

pub fn run_zi(ctx: &ExecutionContext) -> Result<()> {
    let zsh = require("zsh")?;
    let zshrc = zshrc().require()?;

    HOME_DIR.join(".zi").require()?;

    print_separator("zi");

    let cmd = format!("source {} && zi self-update && zi update --all", zshrc.display());
    ctx.run_type().execute(zsh).args(["-i", "-c", &cmd]).status_checked()
}

pub fn run_zim(ctx: &ExecutionContext) -> Result<()> {
    let zsh = require("zsh")?;
    env::var("ZIM_HOME")
        .or_else(|_| {
            Command::new("zsh")
                // TODO: Should these be quoted?
                .args(["-c", "[[ -n ${ZIM_HOME} ]] && print -n ${ZIM_HOME}"])
                .output_checked_utf8()
                .map(|o| o.stdout)
        })
        .map(PathBuf::from)
        .unwrap_or_else(|_| HOME_DIR.join(".zim"))
        .require()?;

    print_separator("zim");

    ctx.run_type()
        .execute(zsh)
        .args(["-i", "-c", "zimfw upgrade && zimfw update"])
        .status_checked()
}

pub fn run_oh_my_zsh(ctx: &ExecutionContext) -> Result<()> {
    require("zsh")?;

    // When updating `oh-my-zsh` on a remote machine through topgrade, the
    // following processes will be created:
    //
    // SSH -> ZSH -> ZSH ($SHELL) -> topgrade -> ZSH
    //
    // The first ZSH process, won't source zshrc (as it is a login shell),
    // and thus it won't have the ZSH environment variable, as a result, the
    // children processes won't get it either, so we source the zshrc and set
    // the ZSH variable for topgrade here.
    if ctx.under_ssh() {
        let res_env_zsh = Command::new("zsh")
            .args(["-ic", "print -rn -- ${ZSH:?}"])
            .output_checked_utf8();

        // this command will fail if `ZSH` is not set
        if let Ok(output) = res_env_zsh {
            let env_zsh = output.stdout;
            debug!("Oh-my-zsh: under SSH, setting ZSH={}", env_zsh);
            env::set_var("ZSH", env_zsh);
        }
    }

    let oh_my_zsh = env::var("ZSH")
        .map(PathBuf::from)
        // default to `~/.oh-my-zsh`
        .unwrap_or(HOME_DIR.join(".oh-my-zsh"))
        .require()?;

    print_separator("oh-my-zsh");

    let custom_dir = env::var::<_>("ZSH_CUSTOM")
        .or_else(|_| {
            Command::new("zsh")
                // TODO: Should these be quoted?
                .args(["-c", "test $ZSH_CUSTOM && echo -n $ZSH_CUSTOM"])
                .output_checked_utf8()
                .map(|o| o.stdout)
        })
        .map(PathBuf::from)
        .unwrap_or_else(|e| {
            let default_path = oh_my_zsh.join("custom");
            debug!(
                "Running zsh returned {}. Using default path: {}",
                e,
                default_path.display()
            );
            default_path
        });

    debug!("oh-my-zsh custom dir: {}", custom_dir.display());

    let mut custom_repos = RepoStep::try_new()?;

    for entry in WalkDir::new(custom_dir).max_depth(2) {
        let entry = entry?;
        custom_repos.insert_if_repo(entry.path());
    }

    custom_repos.remove(&oh_my_zsh);
    ctx.run_type()
        .execute("zsh")
        .arg(&oh_my_zsh.join("tools/upgrade.sh"))
        // oh-my-zsh returns 80 when it is already updated and no changes pulled
        // in this update.
        // See this comment: https://github.com/r-darwish/topgrade/issues/569#issuecomment-736756731
        // for more information.
        .status_checked_with_codes(&[80])
}
