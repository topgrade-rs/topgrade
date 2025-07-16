use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use color_eyre::eyre::Context;
use color_eyre::eyre::{eyre, Result};
use console::style;
use futures::stream::{iter, FuturesUnordered, StreamExt};
use glob::{glob_with, MatchOptions};
use tokio::process::Command as AsyncCommand;
use tokio::runtime;
use tracing::{debug, error};

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::steps::emacs::Emacs;
use crate::terminal::print_separator;
use crate::utils::{require, PathExt};
use crate::{error::SkipStep, terminal::print_warning, HOME_DIR};
use etcetera::base_strategy::BaseStrategy;
use rust_i18n::t;

#[cfg(unix)]
use crate::XDG_DIRS;

#[cfg(windows)]
use crate::WINDOWS_DIRS;

pub fn run_git_pull(ctx: &ExecutionContext) -> Result<()> {
    let mut repos = RepoStep::try_new()?;
    let config = ctx.config();

    // handle built-in repos
    if config.use_predefined_git_repos() {
        // should be executed on all the platforms
        {
            if config.should_run(Step::Emacs) {
                let emacs = Emacs::new();
                if !emacs.is_doom() {
                    if let Some(directory) = emacs.directory() {
                        repos.insert_if_repo(directory);
                    }
                }
                repos.insert_if_repo(HOME_DIR.join(".doom.d"));
            }

            if config.should_run(Step::Vim) {
                repos.insert_if_repo(HOME_DIR.join(".vim"));
                repos.insert_if_repo(HOME_DIR.join(".config/nvim"));
            }

            repos.insert_if_repo(HOME_DIR.join(".ideavimrc"));
            repos.insert_if_repo(HOME_DIR.join(".intellimacs"));

            if config.should_run(Step::Rcm) {
                repos.insert_if_repo(HOME_DIR.join(".dotfiles"));
            }

            let powershell = crate::steps::powershell::Powershell::new();
            if let Some(profile) = powershell.profile() {
                repos.insert_if_repo(profile);
            }
        }

        #[cfg(unix)]
        {
            repos.insert_if_repo(crate::steps::zsh::zshrc());
            if config.should_run(Step::Tmux) {
                repos.insert_if_repo(HOME_DIR.join(".tmux"));
            }
            repos.insert_if_repo(HOME_DIR.join(".config/fish"));
            repos.insert_if_repo(XDG_DIRS.config_dir().join("openbox"));
            repos.insert_if_repo(XDG_DIRS.config_dir().join("bspwm"));
            repos.insert_if_repo(XDG_DIRS.config_dir().join("i3"));
            repos.insert_if_repo(XDG_DIRS.config_dir().join("sway"));
        }

        #[cfg(windows)]
        {
            repos.insert_if_repo(
                WINDOWS_DIRS
                    .cache_dir()
                    .join("Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState"),
            );

            super::os::windows::insert_startup_scripts(&mut repos).ok();
        }
    }

    // Handle user-defined repos
    if let Some(custom_git_repos) = config.git_repos() {
        for git_repo in custom_git_repos {
            repos.glob_insert(git_repo);
        }
    }

    // Warn the user about the bad patterns.
    //
    // NOTE: this should be executed **before** skipping the Git step or the
    // user won't receive this warning in the cases where all the paths configured
    // are bad patterns.
    repos.bad_patterns.iter().for_each(|pattern| {
        print_warning(t!(
            "Path {pattern} did not contain any git repositories",
            pattern = pattern
        ));
    });

    if repos.is_repos_empty() {
        return Err(SkipStep(t!("No repositories to pull").to_string()).into());
    }

    print_separator(t!("Git repositories"));

    repos.pull_repos(ctx)
}

#[cfg(windows)]
static PATH_PREFIX: &str = "\\\\?\\";

pub struct RepoStep {
    git: PathBuf,
    repos: HashSet<PathBuf>,
    glob_match_options: MatchOptions,
    bad_patterns: Vec<String>,
}

#[track_caller]
fn output_checked_utf8(output: Output) -> Result<()> {
    if !(output.status.success()) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        Err(eyre!("{stderr}"))
    } else {
        Ok(())
    }
}

fn get_head_revision<P: AsRef<Path>>(git: &Path, repo: P) -> Option<String> {
    Command::new(git)
        .stdin(Stdio::null())
        .current_dir(repo.as_ref())
        .args(["rev-parse", "HEAD"])
        .output_checked_utf8()
        .map(|output| output.stdout.trim().to_string())
        .map_err(|e| {
            error!("Error getting revision for {}: {e}", repo.as_ref().display(),);

            e
        })
        .ok()
}

impl RepoStep {
    /// Try to create a `RepoStep`, fail if `git` is not found.
    pub fn try_new() -> Result<Self> {
        let git = require("git")?;
        let mut glob_match_options = MatchOptions::new();

        if cfg!(windows) {
            glob_match_options.case_sensitive = false;
        }

        Ok(Self {
            git,
            repos: HashSet::new(),
            bad_patterns: Vec::new(),
            glob_match_options,
        })
    }

    /// Try to get the root of the repo specified in `path`.
    pub fn get_repo_root<P: AsRef<Path>>(&self, path: P) -> Option<PathBuf> {
        match path.as_ref().canonicalize() {
            Ok(mut path) => {
                debug_assert!(path.exists());

                if path.is_file() {
                    debug!("{} is a file. Checking {}", path.display(), path.parent()?.display());
                    path = path.parent()?.to_path_buf();
                }

                debug!("Checking if {} is a git repository", path.display());

                #[cfg(windows)]
                let path = {
                    let mut path_string = path.into_os_string().to_string_lossy().into_owned();
                    if path_string.starts_with(PATH_PREFIX) {
                        path_string.replace_range(0..PATH_PREFIX.len(), "");
                    }

                    debug!("Transformed path to {}", path_string);

                    path_string
                };

                let output = Command::new(&self.git)
                    .stdin(Stdio::null())
                    .current_dir(path)
                    .args(["rev-parse", "--show-toplevel"])
                    .output_checked_utf8()
                    .ok()
                    // trim the last newline char
                    .map(|output| PathBuf::from(output.stdout.trim()));

                return output;
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    debug!("{} does not exist", path.as_ref().display());
                } else {
                    error!("Error looking for {}: {e}", path.as_ref().display());
                }
            }
        }

        None
    }

    /// Check if `path` is a git repo, if yes, add it to `self.repos`.
    ///
    /// Return the check result.
    pub fn insert_if_repo<P: AsRef<Path>>(&mut self, path: P) -> bool {
        if let Some(repo) = self.get_repo_root(path) {
            self.repos.insert(repo);
            true
        } else {
            false
        }
    }

    /// Check if `repo` has a remote.
    fn has_remotes<P: AsRef<Path>>(&self, repo: P) -> Option<bool> {
        let mut cmd = Command::new(&self.git);
        cmd.stdin(Stdio::null())
            .current_dir(repo.as_ref())
            .args(["remote", "show"]);

        let res = cmd.output_checked_utf8();

        res.map(|output| output.stdout.lines().count() > 0)
            .map_err(|e| {
                error!("Error getting remotes for {}: {e}", repo.as_ref().display());
                e
            })
            .ok()
    }

    /// Similar to `insert_if_repo`, with glob support.
    pub fn glob_insert(&mut self, pattern: &str) {
        if let Ok(glob) = glob_with(pattern, self.glob_match_options) {
            let mut last_git_repo: Option<PathBuf> = None;
            for entry in glob {
                match entry {
                    Ok(path) => {
                        if let Some(last_git_repo) = &last_git_repo {
                            if path.is_descendant_of(last_git_repo) {
                                debug!(
                                    "Skipping {} because it's a descendant of last known repo {}",
                                    path.display(),
                                    last_git_repo.display()
                                );
                                continue;
                            }
                        }
                        if self.insert_if_repo(&path) {
                            last_git_repo = Some(path);
                        }
                    }
                    Err(e) => {
                        error!("Error in path {e}");
                    }
                }
            }

            if last_git_repo.is_none() {
                self.bad_patterns.push(String::from(pattern));
            }
        } else {
            error!("Bad glob pattern: {pattern}");
        }
    }

    /// True if `self.repos` is empty.
    pub fn is_repos_empty(&self) -> bool {
        self.repos.is_empty()
    }

    /// Remove `path` from `self.repos`.
    ///
    // `cfg(unix)` because it is only used in the oh-my-zsh step.
    #[cfg(unix)]
    pub fn remove<P: AsRef<Path>>(&mut self, path: P) {
        let _removed = self.repos.remove(path.as_ref());
        debug_assert!(_removed);
    }

    /// Try to pull a repo.
    async fn pull_repo<P: AsRef<Path>>(&self, ctx: &ExecutionContext<'_>, repo: P) -> Result<()> {
        let before_revision = get_head_revision(&self.git, &repo);

        if ctx.config().verbose() {
            println!("{} {}", style(t!("Pulling")).cyan().bold(), repo.as_ref().display());
        }

        let mut command = AsyncCommand::new(&self.git);

        command
            .stdin(Stdio::null())
            .current_dir(&repo)
            .args(["pull", "--ff-only"]);

        if let Some(extra_arguments) = ctx.config().git_arguments() {
            command.args(extra_arguments.split_whitespace());
        }

        let pull_output = command.output().await?;
        let submodule_output = AsyncCommand::new(&self.git)
            .args(["submodule", "update", "--recursive"])
            .current_dir(&repo)
            .stdin(Stdio::null())
            .output()
            .await?;
        let result = output_checked_utf8(pull_output)
            .and_then(|()| output_checked_utf8(submodule_output))
            .wrap_err_with(|| format!("Failed to pull {}", repo.as_ref().display()));

        if result.is_err() {
            println!(
                "{} {} {}",
                style(t!("Failed")).red().bold(),
                t!("pulling"),
                repo.as_ref().display()
            );
        } else {
            let after_revision = get_head_revision(&self.git, repo.as_ref());

            match (&before_revision, &after_revision) {
                (Some(before), Some(after)) if before != after => {
                    println!("{} {}", style(t!("Changed")).yellow().bold(), repo.as_ref().display());

                    Command::new(&self.git)
                        .stdin(Stdio::null())
                        .current_dir(&repo)
                        .args([
                            "--no-pager",
                            "log",
                            "--no-decorate",
                            "--oneline",
                            &format!("{before}..{after}"),
                        ])
                        .status_checked()?;
                    println!();
                }
                _ => {
                    if ctx.config().verbose() {
                        println!("{} {}", style(t!("Up-to-date")).green().bold(), repo.as_ref().display());
                    }
                }
            }
        }

        result
    }

    /// Pull the repositories specified in `self.repos`.
    ///
    /// # NOTE
    /// This function will create an async runtime and do the real job so the
    /// function itself is not async.
    fn pull_repos(&self, ctx: &ExecutionContext) -> Result<()> {
        if ctx.run_type().dry() {
            self.repos
                .iter()
                .for_each(|repo| println!("{}", t!("Would pull {repo}", repo = repo.display())));

            return Ok(());
        }

        if !ctx.config().verbose() {
            println!(
                "\n{} {}\n",
                style(t!("Only")).green().bold(),
                t!("updated repositories will be shown...")
            );
        }

        let futures_iterator = self
            .repos
            .iter()
            .filter(|repo| match self.has_remotes(repo) {
                Some(false) => {
                    println!(
                        "{} {} {}",
                        style(t!("Skipping")).yellow().bold(),
                        repo.display(),
                        t!("because it has no remotes")
                    );
                    false
                }
                _ => true, // repo has remotes or command to check for remotes has failed. proceed to pull anyway.
            })
            .map(|repo| self.pull_repo(ctx, repo));

        let stream_of_futures = if let Some(limit) = ctx.config().git_concurrency_limit() {
            iter(futures_iterator).buffer_unordered(limit).boxed()
        } else {
            futures_iterator.collect::<FuturesUnordered<_>>().boxed()
        };

        let basic_rt = runtime::Runtime::new()?;
        let results = basic_rt.block_on(async { stream_of_futures.collect::<Vec<Result<()>>>().await });

        let error = results.into_iter().find(std::result::Result::is_err);
        error.unwrap_or(Ok(()))
    }
}
