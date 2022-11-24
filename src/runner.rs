use crate::ctrlc;
use crate::error::{DryRun, SkipStep};
use crate::execution_context::ExecutionContext;
use crate::report::{Report, StepResult};
use crate::terminal::print_error;
use crate::{config::Step, terminal::should_retry};
use color_eyre::eyre::Result;
use std::borrow::Cow;
use std::fmt::Debug;
use tracing::debug;

pub struct Runner<'a> {
    ctx: &'a ExecutionContext<'a>,
    report: Report<'a>,
}

impl<'a> Runner<'a> {
    pub fn new(ctx: &'a ExecutionContext) -> Runner<'a> {
        Runner {
            ctx,
            report: Report::new(),
        }
    }

    pub fn execute<F, M>(&mut self, step: Step, key: M, func: F) -> Result<()>
    where
        F: Fn() -> Result<()>,
        M: Into<Cow<'a, str>> + Debug,
    {
        if !self.ctx.config().should_run(step) {
            return Ok(());
        }

        let key = key.into();
        debug!("Step {:?}", key);

        loop {
            match func() {
                Ok(()) => {
                    self.report.push_result(Some((key, StepResult::Success)));
                    break;
                }
                Err(e) if e.downcast_ref::<DryRun>().is_some() => break,
                Err(e) if e.downcast_ref::<SkipStep>().is_some() => {
                    if self.ctx.config().verbose() || self.ctx.config().show_skipped() {
                        self.report.push_result(Some((key, StepResult::Skipped(e.to_string()))));
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
                        self.report.push_result(Some((
                            key,
                            if ignore_failure {
                                StepResult::Ignored
                            } else {
                                StepResult::Failure
                            },
                        )));
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn report(&self) -> &Report {
        &self.report
    }
}
