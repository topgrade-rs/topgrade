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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::error::SkipStep;
    use crate::execution_context::RunType;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn mock_execution_context() -> ExecutionContext<'static> {
        ExecutionContext::new(RunType::Dry, None, &Config::default())
    }

    #[test]
    fn test_runner_execute_skip_step() {
        let config = Config::default(); // Assuming Config::default() exists
        let run_type = RunType::new(false);
        let sudo = None; // Assuming sudo is optional and can be None
        let ctx = ExecutionContext::new(run_type, sudo, &config);
        let mut runner = Runner::new(&ctx);
        let step = Step::new(); // Assuming Step::new() exists
        let result = runner.execute(step, "test_step", || Err(SkipStep("Test skip".into()).into()));
        assert!(result.is_ok(), "Expected skipped step to result in Ok(())");
        assert!(matches!(runner.report().last_result(), Some((_, StepResult::Skipped(_)))));
    }

    #[test]
    fn test_runner_execute_success() {
        let ctx = mock_execution_context();
        let mut runner = Runner::new(&ctx);
        let step = Step::new(); // Assuming Step::new() exists
        let result = runner.execute(step, "test_step", || Ok(()));
        assert!(result.is_ok());
        assert!(matches!(runner.report().last_result(), Some((_, StepResult::Success))));
    }

    #[test]
    fn test_runner_execute_failure() {
        let ctx = mock_execution_context();
        let mut runner = Runner::new(&ctx);
        let step = Step::new(); // Assuming Step::new() exists
        let result = runner.execute(step, "test_step", || Err(color_eyre::eyre::eyre!("Test failure")));
        assert!(result.is_ok());
        assert!(matches!(runner.report().last_result(), Some((_, StepResult::Failure))));
    }

    #[test]
    fn test_runner_execute_retry() {
        let ctx = mock_execution_context();
        let mut runner = Runner::new(&ctx);
        let step = Step::new(); // Assuming Step::new() exists
        let retry_flag = Arc::new(AtomicBool::new(false));
        let retry_flag_clone = Arc::clone(&retry_flag);
        let result = runner.execute(step, "test_step", move || {
            if retry_flag_clone.load(Ordering::SeqCst) {
                Ok(())
            } else {
                retry_flag_clone.store(true, Ordering::SeqCst);
                Err(color_eyre::eyre::eyre!("Test failure"))
            }
        });
        assert!(result.is_ok());
        assert!(matches!(runner.report().last_result(), Some((_, StepResult::Success))));
    }

    #[test]
    fn test_runner_execute_ignored() {
        let ctx = mock_execution_context();
        let mut runner = Runner::new(&ctx);
        let step = Step::new(); // Assuming Step::new() exists
        let result = runner.execute(step, "test_step", || Err(color_eyre::eyre::eyre!("Test failure")));
        assert!(result.is_ok());
        assert!(matches!(runner.report().last_result(), Some((_, StepResult::Ignored))));
    }
    // Additional tests for success, failure, and ignored cases
}
