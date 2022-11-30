use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::Result;
use directories::BaseDirs;
use tracing::debug;
use walkdir::WalkDir;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::executor::RunType;
use crate::git::Repositories;
use crate::terminal::print_separator;
use crate::utils::{require, PathExt};

pub fn run_zr(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let zsh = require("zsh")?;

    require("zr")?;

    print_separator("zr");

    let cmd = format!("source {} && zr --update", zshrc(base_dirs).display());
    run_type.execute(zsh).args(["-l", "-c", cmd.as_str()]).status_checked()
}

pub fn zshrc(base_dirs: &BaseDirs) -> PathBuf {
    env::var("ZDOTDIR")
        .map(|p| Path::new(&p).join(".zshrc"))
        .unwrap_or_else(|_| base_dirs.home_dir().join(".zshrc"))
}

pub fn run_antibody(run_type: RunType) -> Result<()> {
    require("zsh")?;
    let antibody = require("antibody")?;

    print_separator("antibody");

    run_type.execute(antibody).arg("update").status_checked()
}

pub fn run_antigen(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let zsh = require("zsh")?;
    let zshrc = zshrc(base_dirs).require()?;
    env::var("ADOTDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base_dirs.home_dir().join("antigen.zsh"))
        .require()?;

    print_separator("antigen");

    let cmd = format!("source {} && (antigen selfupdate ; antigen update)", zshrc.display());
    run_type.execute(zsh).args(["-l", "-c", cmd.as_str()]).status_checked()
}

pub fn run_zgenom(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let zsh = require("zsh")?;
    let zshrc = zshrc(base_dirs).require()?;
    env::var("ZGEN_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base_dirs.home_dir().join(".zgenom"))
        .require()?;

    print_separator("zgenom");

    let cmd = format!("source {} && zgenom selfupdate && zgenom update", zshrc.display());
    run_type.execute(zsh).args(["-l", "-c", cmd.as_str()]).status_checked()
}

pub fn run_zplug(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let zsh = require("zsh")?;
    zshrc(base_dirs).require()?;

    env::var("ZPLUG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base_dirs.home_dir().join(".zplug"))
        .require()?;

    print_separator("zplug");

    run_type
        .execute(zsh)
        .args(["-i", "-c", "zplug update"])
        .status_checked()
}

pub fn run_zinit(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let zsh = require("zsh")?;
    let zshrc = zshrc(base_dirs).require()?;

    env::var("ZINIT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base_dirs.home_dir().join(".zinit"))
        .require()?;

    print_separator("zinit");

    let cmd = format!("source {} && zinit self-update && zinit update --all", zshrc.display(),);
    run_type.execute(zsh).args(["-i", "-c", cmd.as_str()]).status_checked()
}

pub fn run_zi(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let zsh = require("zsh")?;
    let zshrc = zshrc(base_dirs).require()?;

    base_dirs.home_dir().join(".zi").require()?;

    print_separator("zi");

    let cmd = format!("source {} && zi self-update && zi update --all", zshrc.display(),);
    run_type.execute(zsh).args(["-i", "-c", &cmd]).status_checked()
}

pub fn run_zim(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
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
        .unwrap_or_else(|_| base_dirs.home_dir().join(".zim"))
        .require()?;

    print_separator("zim");

    run_type
        .execute(zsh)
        .args(["-i", "-c", "zimfw upgrade && zimfw update"])
        .status_checked()
}

pub fn run_oh_my_zsh(ctx: &ExecutionContext) -> Result<()> {
    require("zsh")?;
    let oh_my_zsh = ctx.base_dirs().home_dir().join(".oh-my-zsh").require()?;

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

    let mut custom_repos = Repositories::new(ctx.git());

    for entry in WalkDir::new(custom_dir).max_depth(2) {
        let entry = entry?;
        custom_repos.insert_if_repo(entry.path());
    }

    custom_repos.remove(&oh_my_zsh.to_string_lossy());
    if !custom_repos.is_empty() {
        println!("Pulling custom plugins and themes");
        ctx.git().multi_pull(&custom_repos, ctx)?;
    }

    ctx.run_type()
        .execute("zsh")
        .env("ZSH", &oh_my_zsh)
        .arg(&oh_my_zsh.join("tools/upgrade.sh"))
        .status_checked_with_codes(&[80])
}
