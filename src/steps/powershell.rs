use std::path::PathBuf;

use color_eyre::eyre::Result;
#[cfg(windows)]
use color_eyre::eyre::eyre;
use tracing::debug;

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::executor::Executor;
use crate::terminal;
use crate::utils::{PathExt, which};

pub struct Powershell {
    path: PathBuf,
    profile: Option<PathBuf>,
    is_pwsh: bool,
}

impl Powershell {
    pub fn new(ctx: &ExecutionContext) -> Option<Self> {
        if terminal::is_dumb() {
            return None;
        }

        let (path, is_pwsh) = which("pwsh")
            .map(|p| (Some(p), true))
            .or_else(|| which("powershell").map(|p| (Some(p), false)))
            .unwrap_or((None, false));

        path.map(|path| {
            let mut ret = Self {
                path,
                profile: None,
                is_pwsh,
            };
            ret.set_profile(ctx);
            ret
        })
    }

    pub fn profile(&self) -> Option<&PathBuf> {
        self.profile.as_ref()
    }

    fn set_profile(&mut self, ctx: &ExecutionContext) {
        let profile = self
            .build_command_internal(ctx, "Split-Path $PROFILE")
            .output_checked_utf8()
            .map(|output| output.stdout.trim().to_string())
            .and_then(|s| PathBuf::from(s).require())
            .ok();
        debug!("Found PowerShell profile: {:?}", profile);
        self.profile = profile;
    }

    pub fn is_pwsh(&self) -> bool {
        self.is_pwsh
    }

    /// Builds an "internal" powershell command
    pub fn build_command_internal(&self, ctx: &ExecutionContext, cmd: &str) -> Executor {
        let mut command = ctx.execute(&self.path).always();

        command.args(["-NoProfile", "-Command"]);
        command.arg(cmd);

        // If topgrade was run from pwsh, but we are trying to run powershell, then
        // the inherited PSModulePath breaks module imports
        if !self.is_pwsh {
            command.env_remove("PSModulePath");
        }

        command
    }

    /// Builds a "primary" powershell command (uses dry-run if required):
    /// {powershell} -NoProfile -Command {cmd}
    pub fn build_command<'a>(
        &self,
        ctx: &'a ExecutionContext,
        cmd: &str,
        use_sudo: bool,
    ) -> Result<impl CommandExt + 'a> {
        let mut command = if use_sudo {
            let sudo = ctx.require_sudo()?;
            sudo.execute(ctx, &self.path)?
        } else {
            ctx.execute(&self.path)
        };

        #[cfg(windows)]
        {
            // Check execution policy and return early if it's not set correctly
            self.execution_policy_args_if_needed(ctx)?;
        }

        command.args(["-NoProfile", "-Command"]);
        command.arg(cmd);

        // If topgrade was run from pwsh, but we are trying to run powershell, then
        // the inherited PSModulePath breaks module imports
        if !self.is_pwsh {
            command.env_remove("PSModulePath");
        }

        Ok(command)
    }

    #[cfg(windows)]
    fn execution_policy_args_if_needed(&self, ctx: &ExecutionContext) -> Result<()> {
        if !self.is_execution_policy_set(ctx, "RemoteSigned") {
            Err(eyre!(
                "PowerShell execution policy is too restrictive. \
                Please run 'Set-ExecutionPolicy RemoteSigned -Scope CurrentUser' in PowerShell \
                (or use Unrestricted/Bypass if you're sure about the security implications)"
            ))
        } else {
            Ok(())
        }
    }

    #[cfg(windows)]
    fn is_execution_policy_set(&self, ctx: &ExecutionContext, policy: &str) -> bool {
        // These policies are ordered from most restrictive to least restrictive
        let valid_policies = ["Restricted", "AllSigned", "RemoteSigned", "Unrestricted", "Bypass"];

        // Find the index of our target policy
        let target_idx = valid_policies.iter().position(|&p| p == policy);

        let current_policy = self
            .build_command_internal(ctx, "Get-ExecutionPolicy")
            .output_checked_utf8()
            .map(|output| output.stdout.trim().to_string());

        debug!("Found PowerShell ExecutionPolicy: {:?}", current_policy);

        current_policy.is_ok_and(|current_policy| {
            // Find the index of the current policy
            let current_idx = valid_policies.iter().position(|&p| p == current_policy);

            // Check if current policy exists and is at least as permissive as the target
            match (current_idx, target_idx) {
                (Some(current), Some(target)) => current >= target,
                _ => false,
            }
        })
    }

    #[cfg(windows)]
    pub fn has_module(&self, ctx: &ExecutionContext, module_name: &str) -> bool {
        let cmd = format!("Get-Module -ListAvailable {}", module_name);

        self.build_command_internal(ctx, &cmd)
            .output_checked()
            .map(|output| !output.stdout.trim_ascii().is_empty())
            .unwrap_or(false)
    }
}
