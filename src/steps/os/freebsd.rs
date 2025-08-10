use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::terminal::print_separator;
use color_eyre::Result;
use rust_i18n::t;

pub fn upgrade_freebsd(ctx: &ExecutionContext) -> Result<()> {
    let sudo = ctx.require_sudo()?;
    print_separator(t!("FreeBSD Update"));
    sudo.execute(ctx, "/usr/sbin/freebsd-update")?
        .args(["fetch", "install"])
        .status_checked()
}

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = ctx.require_sudo()?;
    print_separator(t!("FreeBSD Packages"));

    let mut command = sudo.execute(ctx, "/usr/sbin/pkg")?;
    command.arg("upgrade");
    if ctx.config().yes(Step::System) {
        command.arg("-y");
    }
    command.status_checked()
}

pub fn audit_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = ctx.require_sudo()?;

    print_separator(t!("FreeBSD Audit"));

    sudo.execute(ctx, "/usr/sbin/pkg")?
        .args(["audit", "-Fr"])
        .status_checked()
}
