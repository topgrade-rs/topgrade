use color_eyre::eyre::{Result, WrapErr};
use rust_i18n::t;
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::io;
use tracing::debug;

use crate::ctrlc;
use crate::error::{DryRun, MissingSudo, SkipStep};
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::{print_error, print_warning, should_retry, ShouldRetry};

pub enum StepResult {
    Success(Option<UpdatedComponents>),
    Failure,
    Ignored,
    SkippedMissingSudo,
    Skipped(String),
}

impl StepResult {
    pub fn failed(&self) -> bool {
        use StepResult::*;

        match self {
            Success(_) | Ignored | Skipped(_) | SkippedMissingSudo => false,
            Failure => true,
        }
    }
}

enum RetryDecision {
    Retry,
    Quit,
    Continue(StepResult),
}

type Report<'a> = Vec<(Cow<'a, str>, StepResult)>;

pub struct UpdatedComponents(Vec<UpdatedComponent>);

impl UpdatedComponents {
    pub fn new(updated: Vec<UpdatedComponent>) -> Self {
        Self(updated)
    }
}

impl Display for UpdatedComponents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.as_slice() {
            [] => write!(f, "No updates found"),
            components => {
                writeln!(f, "Updated:")?;
                let updates = components
                    .iter()
                    .map(|c| format!("- {c}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                write!(f, "{updates}")?;
                Ok(())
            }
        }
    }
}

pub struct UpdatedComponent {
    name: String,
    from_version: Option<String>,
    to_version: Option<String>,
}

impl UpdatedComponent {
    pub fn new(name: String, from_version: Option<String>, to_version: Option<String>) -> Self {
        Self {
            name,
            from_version,
            to_version,
        }
    }
}

impl Display for UpdatedComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.from_version, &self.to_version) {
            (None, None) => write!(f, "{}", self.name),
            (None, Some(to_version)) => write!(f, "{} to {}", self.name, to_version),
            (Some(from_version), None) => write!(f, "{} from {}", self.name, from_version),
            (Some(from_version), Some(to_version)) => {
                write!(f, "{} from {} to {}", self.name, from_version, to_version)
            }
        }
    }
}

pub struct Runner<'a> {
    ctx: &'a ExecutionContext<'a>,
    report: Report<'a>,
}

impl<'a> Runner<'a> {
    pub fn new(ctx: &'a ExecutionContext) -> Runner<'a> {
        Runner {
            ctx,
            report: Vec::new(),
        }
    }

    fn handle_retry_prompt(
        &self,
        key: &str,
        error: &color_eyre::eyre::Error,
        ignore_failure: bool,
    ) -> Result<RetryDecision> {
        print_error(key, format!("{error:?}"));
        match should_retry(key)? {
            ShouldRetry::Yes => Ok(RetryDecision::Retry),
            ShouldRetry::Quit => Ok(RetryDecision::Quit),
            ShouldRetry::No => Ok(RetryDecision::Continue(if ignore_failure {
                StepResult::Ignored
            } else {
                StepResult::Failure
            })),
        }
    }

    fn push_result(&mut self, key: Cow<'a, str>, result: StepResult) {
        debug_assert!(!self.report.iter().any(|(k, _)| k == &key), "{key} already reported");
        self.report.push((key, result));
    }

    pub fn execute<K, F>(&mut self, step: Step, key: K, func: F) -> Result<()>
    where
        K: Into<Cow<'a, str>> + Debug,
        F: Fn() -> Result<()>,
    {
        self._execute(step, key, || func().map(|()| None))
    }

    pub fn execute_with_updated<K, F>(&mut self, step: Step, key: K, func: F) -> Result<()>
    where
        K: Into<Cow<'a, str>> + Debug,
        F: Fn() -> Result<Vec<UpdatedComponent>>,
    {
        self._execute(step, key, || func().map(Some))
    }

    fn _execute<K, F>(&mut self, step: Step, key: K, func: F) -> Result<()>
    where
        K: Into<Cow<'a, str>> + Debug,
        F: Fn() -> Result<Option<Vec<UpdatedComponent>>>,
    {
        if !self.ctx.config().should_run(step) {
            return Ok(());
        }

        let key: Cow<'a, str> = key.into();
        debug!("Step {:?}", key);

        // alter the `func` to put it in a span
        let func = || {
            let span =
                tracing::span!(parent: tracing::Span::none(), tracing::Level::TRACE, "step", step = ?step, key = %key);
            let _guard = span.enter();
            func()
        };

        // Total max attempts = 1 (initial) + auto_retry count
        let max_attempts = self.ctx.config().auto_retry().saturating_add(1);

        let mut attempt = 1;

        loop {
            match func() {
                Ok(updated) => {
                    self.push_result(key, StepResult::Success(updated.map(UpdatedComponents::new)));
                    break;
                }
                Err(e) if e.downcast_ref::<DryRun>().is_some() => break,
                Err(e) if e.downcast_ref::<MissingSudo>().is_some() => {
                    print_warning(t!("Skipping step, sudo is required"));
                    self.push_result(key, StepResult::SkippedMissingSudo);
                    break;
                }
                Err(e) if e.downcast_ref::<SkipStep>().is_some() => {
                    if self.ctx.config().verbose() || self.ctx.config().show_skipped() {
                        self.push_result(key, StepResult::Skipped(e.to_string()));
                    }
                    break;
                }
                Err(e) => {
                    debug!("Step {:?} failed: {:?}", key, e);
                    let interrupted = ctrlc::interrupted();
                    if interrupted {
                        ctrlc::unset_interrupted();
                    }

                    let ignore_failure = self.ctx.config().ignore_failure(step);

                    // Decide whether to prompt the user
                    let has_auto_retries_left = attempt < max_attempts;
                    let should_prompt =
						// If interrupted, always prompt
                        interrupted
                            // Otherwise, only prompt when we don't want to auto-retry, are configured
                            //  to prompt, and don't ignore failure for this step (when we ignore_failure,
                            //  we don't prompt regardless of ask_retry)
                            || (!has_auto_retries_left && self.ctx.config().ask_retry() && !ignore_failure);

                    if should_prompt {
                        match self.handle_retry_prompt(&key, &e, ignore_failure)? {
                            RetryDecision::Retry => {
                                continue;
                            }
                            RetryDecision::Quit => {
                                self.push_result(key, StepResult::Failure);
                                return Err(io::Error::from(io::ErrorKind::Interrupted))
                                    .context("Quit from user input");
                            }
                            RetryDecision::Continue(result) => {
                                self.push_result(key, result);
                                break;
                            }
                        }
                    } else if has_auto_retries_left {
                        attempt += 1;
                    } else {
                        self.push_result(
                            key,
                            if ignore_failure {
                                StepResult::Ignored
                            } else {
                                StepResult::Failure
                            },
                        );
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn report(&self) -> &Report<'_> {
        &self.report
    }
}
