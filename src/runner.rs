use color_eyre::eyre::Result;
use std::borrow::Cow;
use std::fmt::Debug;
use tracing::debug;

use crate::ctrlc;
use crate::error::{DryRun, SkipStep};
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::{print_error, should_retry};

pub enum StepResult {
    Success,
    Failure,
    Ignored,
    Skipped(String),
}

impl StepResult {
    pub fn failed(&self) -> bool {
        use StepResult::*;

        match self {
            Success | Ignored | Skipped(_) => false,
            Failure => true,
        }
    }
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

        loop {
            match func() {
                Ok(()) => {
                    self.push_result(key, StepResult::Success);
                    break;
                }
                Err(e) if e.downcast_ref::<DryRun>().is_some() => break,
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
                    let should_ask = interrupted || !(self.ctx.config().no_retry() || ignore_failure);
                    let should_retry = if should_ask {
                        print_error(&key, format!("{e:?}"));
                        should_retry(interrupted, key.as_ref())?
                    } else {
                        false
                    };

                    if !should_retry {
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
