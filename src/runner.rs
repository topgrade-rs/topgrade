use color_eyre::eyre::{Result, WrapErr};
use rust_i18n::t;
use std::borrow::Cow;
use std::fmt::Debug;
use std::io;
use tracing::debug;

use crate::ctrlc;
use crate::error::{DryRun, MissingSudo, SkipStep};
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::{print_error, print_warning, should_retry, ShouldRetry};

pub enum StepResult {
    Success,
    Failure,
    Ignored,
    SkippedMissingSudo,
    Skipped(String),
}

impl StepResult {
    pub fn failed(&self) -> bool {
        use StepResult::*;

        match self {
            Success | Ignored | Skipped(_) | SkippedMissingSudo => false,
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
                Ok(()) => {
                    self.push_result(key, StepResult::Success);
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
                    let should_prompt = if interrupted {
                        // If interrupted, always prompt
                        true
                    } else if attempt < max_attempts {
                        // If not interrupted, auto-retry if we want to
                        attempt += 1;
                        continue;
                    } else if ignore_failure {
                        // If ignore_failure, never prompt (except when interrupted)
                        false
                    } else {
                        // Otherwise, prompt when we are configured to prompt
                        self.ctx.config().ask_retry()
                    };

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
