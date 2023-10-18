use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use color_eyre::eyre::Context;
use color_eyre::eyre::{eyre, Result};
use console::style;
use futures::stream::{iter, FuturesUnordered};
use futures::StreamExt;
use glob::{glob_with, MatchOptions};
use tokio::process::Command as AsyncCommand;
use tokio::runtime;
use tracing::{debug, error};

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::utils::{which, PathExt};
use crate::{error::SkipStep, terminal::print_warning};

#[cfg(windows)]
static PATH_PREFIX: &str = "\\\\?\\";

#[derive(Debug)]
pub struct Git {
    git: Option<PathBuf>,
}

#[derive(Clone, Copy)]
pub enum GitAction {
    Push,
    Pull,
}

#[derive(Debug)]
pub struct Repositories<'a> {
    git: &'a Git,
    pull_repositories: HashSet<String>,
    push_repositories: HashSet<String>,
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
async fn push_repository(repo: String, git: &Path, ctx: &ExecutionContext<'_>) -> Result<()> {
    let path = repo.to_string();

    println!("{} {}", style("Pushing").cyan().bold(), path);

    let mut command = AsyncCommand::new(git);

    command
        .stdin(Stdio::null())
        .current_dir(&repo)
        .args(["push", "--porcelain"]);
    if let Some(extra_arguments) = ctx.config().push_git_arguments() {
        command.args(extra_arguments.split_whitespace());
    }

    let output = command.output().await?;
    let result = match output.status.success() {
        true => Ok(()),
        false => Err(format!("Failed to push {repo}")),
    };

    if result.is_err() {
        println!("{} pushing {}", style("Failed").red().bold(), &repo);
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(eyre!(e)),
    }
}

async fn pull_repository(repo: String, git: &Path, ctx: &ExecutionContext<'_>) -> Result<()> {
    let path = repo.to_string();
    let before_revision = get_head_revision(git, &repo);

    println!("{} {}", style("Pulling").cyan().bold(), path);

    let mut command = AsyncCommand::new(git);

    command
        .stdin(Stdio::null())
        .current_dir(&repo)
        .args(["pull", "--ff-only"]);

    if let Some(extra_arguments) = ctx.config().pull_git_arguments() {
        command.args(extra_arguments.split_whitespace());
    }

    let pull_output = command.output().await?;
    let submodule_output = AsyncCommand::new(git)
        .args(["submodule", "update", "--recursive"])
        .current_dir(&repo)
        .stdin(Stdio::null())
        .output()
        .await?;
    let result = output_checked_utf8(pull_output)
        .and_then(|_| output_checked_utf8(submodule_output))
        .wrap_err_with(|| format!("Failed to pull {repo}"));

    if result.is_err() {
        println!("{} pulling {}", style("Failed").red().bold(), &repo);
    } else {
        let after_revision = get_head_revision(git, &repo);

        match (&before_revision, &after_revision) {
            (Some(before), Some(after)) if before != after => {
                println!("{} {}:", style("Changed").yellow().bold(), &repo);

                Command::new(git)
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
                println!("{} {}", style("Up-to-date").green().bold(), &repo);
            }
        }
    }

    result.map(|_| ())
}

fn get_head_revision(git: &Path, repo: &str) -> Option<String> {
    Command::new(git)
        .stdin(Stdio::null())
        .current_dir(repo)
        .args(["rev-parse", "HEAD"])
        .output_checked_utf8()
        .map(|output| output.stdout.trim().to_string())
        .map_err(|e| {
            error!("Error getting revision for {}: {}", repo, e);

            e
        })
        .ok()
}

fn has_remotes(git: &Path, repo: &str) -> Option<bool> {
    Command::new(git)
        .stdin(Stdio::null())
        .current_dir(repo)
        .args(["remote", "show"])
        .output_checked_utf8()
        .map(|output| output.stdout.lines().count() > 0)
        .map_err(|e| {
            error!("Error getting remotes for {}: {}", repo, e);
            e
        })
        .ok()
}

impl Git {
    pub fn new() -> Self {
        Self { git: which("git") }
    }

    pub fn get_repo_root<P: AsRef<Path>>(&self, path: P) -> Option<String> {
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

                if let Some(git) = &self.git {
                    let output = Command::new(git)
                        .stdin(Stdio::null())
                        .current_dir(path)
                        .args(["rev-parse", "--show-toplevel"])
                        .output_checked_utf8()
                        .ok()
                        .map(|output| output.stdout.trim().to_string());
                    return output;
                }
            }
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => debug!("{} does not exist", path.as_ref().display()),
                _ => error!("Error looking for {}: {}", path.as_ref().display(), e),
            },
        }

        None
    }
    pub fn multi_repo_step(&self, repositories: &Repositories, ctx: &ExecutionContext) -> Result<()> {
        // Warn the user about the bad patterns.
        //
        // NOTE: this should be executed **before** skipping the Git step or the
        // user won't receive this warning in the cases where all the paths configured
        // are bad patterns.
        repositories
            .bad_patterns
            .iter()
            .for_each(|pattern| print_warning(format!("Path {pattern} did not contain any git repositories")));

        if repositories.is_empty() {
            return Err(SkipStep(String::from("No repositories to pull or push")).into());
        }

        print_separator("Git repositories");
        self.multi_pull(repositories, ctx)?;
        self.multi_push(repositories, ctx)?;

        Ok(())
    }

    pub fn multi_pull(&self, repositories: &Repositories, ctx: &ExecutionContext) -> Result<()> {
        let git = self.git.as_ref().unwrap();

        if ctx.run_type().dry() {
            repositories
                .pull_repositories
                .iter()
                .for_each(|repo| println!("Would pull {}", &repo));

            return Ok(());
        }

        let futures_iterator = repositories
            .pull_repositories
            .iter()
            .filter(|repo| match has_remotes(git, repo) {
                Some(false) => {
                    println!(
                        "{} {} because it has no remotes",
                        style("Skipping").yellow().bold(),
                        repo
                    );
                    false
                }
                _ => true, // repo has remotes or command to check for remotes has failed. proceed to pull anyway.
            })
            .map(|repo| pull_repository(repo.clone(), git, ctx));

        let stream_of_futures = if let Some(limit) = ctx.config().git_concurrency_limit() {
            iter(futures_iterator).buffer_unordered(limit).boxed()
        } else {
            futures_iterator.collect::<FuturesUnordered<_>>().boxed()
        };

        let basic_rt = runtime::Runtime::new()?;
        let results = basic_rt.block_on(async { stream_of_futures.collect::<Vec<Result<()>>>().await });

        let error = results.into_iter().find(|r| r.is_err());
        error.unwrap_or(Ok(()))
    }

    pub fn multi_push(&self, repositories: &Repositories, ctx: &ExecutionContext) -> Result<()> {
        let git = self.git.as_ref().unwrap();

        if ctx.run_type().dry() {
            repositories
                .push_repositories
                .iter()
                .for_each(|repo| println!("Would push {}", &repo));

            return Ok(());
        }

        let futures_iterator = repositories
            .push_repositories
            .iter()
            .filter(|repo| match has_remotes(git, repo) {
                Some(false) => {
                    println!(
                        "{} {} because it has no remotes",
                        style("Skipping").yellow().bold(),
                        repo
                    );
                    false
                }
                _ => true, // repo has remotes or command to check for remotes has failed. proceed to pull anyway.
            })
            .map(|repo| push_repository(repo.clone(), git, ctx));

        let stream_of_futures = if let Some(limit) = ctx.config().git_concurrency_limit() {
            iter(futures_iterator).buffer_unordered(limit).boxed()
        } else {
            futures_iterator.collect::<FuturesUnordered<_>>().boxed()
        };

        let basic_rt = runtime::Runtime::new()?;
        let results = basic_rt.block_on(async { stream_of_futures.collect::<Vec<Result<()>>>().await });

        let error = results.into_iter().find(|r| r.is_err());
        error.unwrap_or(Ok(()))
    }
}

impl<'a> Repositories<'a> {
    pub fn new(git: &'a Git) -> Self {
        let mut glob_match_options = MatchOptions::new();

        if cfg!(windows) {
            glob_match_options.case_sensitive = false;
        }

        Self {
            git,
            bad_patterns: Vec::new(),
            glob_match_options,
            pull_repositories: HashSet::new(),
            push_repositories: HashSet::new(),
        }
    }

    pub fn insert_if_repo<P: AsRef<Path>>(&mut self, path: P, action: GitAction) -> bool {
        if let Some(repo) = self.git.get_repo_root(path) {
            match action {
                GitAction::Push => self.push_repositories.insert(repo),
                GitAction::Pull => self.pull_repositories.insert(repo),
            };

            true
        } else {
            false
        }
    }

    pub fn glob_insert(&mut self, pattern: &str, action: GitAction) {
        if let Ok(glob) = glob_with(pattern, self.glob_match_options) {
            let mut last_git_repo: Option<PathBuf> = None;
            for entry in glob {
                match entry {
                    Ok(path) => {
                        if let Some(last_git_repo) = &last_git_repo {
                            if path.is_descendant_of(last_git_repo) {
                                debug!(
                                    "Skipping {} because it's a decendant of last known repo {}",
                                    path.display(),
                                    last_git_repo.display()
                                );
                                continue;
                            }
                        }
                        if self.insert_if_repo(&path, action) {
                            last_git_repo = Some(path);
                        }
                    }
                    Err(e) => {
                        error!("Error in path {}", e);
                    }
                }
            }

            if last_git_repo.is_none() {
                self.bad_patterns.push(String::from(pattern));
            }
        } else {
            error!("Bad glob pattern: {}", pattern);
        }
    }

    /// Return true if `pull_repos` and `push_repos` are both empty.
    pub fn is_empty(&self) -> bool {
        self.pull_repositories.is_empty() && self.push_repositories.is_empty()
    }

    // The following 2 functions are `#[cfg(unix)]` because they are only used in
    // the `oh-my-zsh` step, which is UNIX-only.

    #[cfg(unix)]
    /// Return true if `pull_repos` is empty.
    pub fn pull_is_empty(&self) -> bool {
        self.pull_repositories.is_empty()
    }

    #[cfg(unix)]
    /// Remove `path` from `pull_repos`
    ///
    /// # Panic
    /// Will panic if `path` is not in the `pull_repos` under a debug build.
    pub fn remove_from_pull(&mut self, path: &str) {
        let _removed = self.pull_repositories.remove(path);
        debug_assert!(_removed);
    }
}
