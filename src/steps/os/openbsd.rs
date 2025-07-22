use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::executor::RunType;
use crate::terminal::print_separator;
use crate::utils::{get_require_sudo_string, require_option};
use color_eyre::eyre::Result;
use rust_i18n::t;
use std::fs;

fn is_openbsd_current(ctx: &ExecutionContext) -> Result<bool> {
    let motd_content = fs::read_to_string("/etc/motd")?;
    let is_current = ["-current", "-beta"].iter().any(|&s| motd_content.contains(s));
    match ctx.config.run_type() {
        RunType::Dry | RunType::Damp => {
            println!("{}", t!("Checking if /etc/motd contains -current or -beta"));
        }
        RunType::Wet => {}
    }
    Ok(is_current)
}

pub fn upgrade_openbsd(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("OpenBSD Update"));

    let is_current = is_openbsd_current(ctx)?;

    let mut cmd = ctx.run_type().execute(sudo);
    if is_current {
        cmd.args(["/usr/sbin/sysupgrade", "-sn"]);
    } else {
        cmd.arg("/usr/sbin/syspatch");
    };

    cmd.status_checked()
}

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), get_require_sudo_string())?;
    print_separator(t!("OpenBSD Packages"));

    let is_current = is_openbsd_current(ctx)?;

    if ctx.config().cleanup() {
        ctx.run_type()
            .execute(sudo)
            .args(["/usr/sbin/pkg_delete", "-ac"])
            .status_checked()?;
    }

    let mut cmd = ctx.run_type().execute(sudo);
    cmd.args(["/usr/sbin/pkg_add", "-u"]);
    if is_current {
        cmd.arg("-Dsnap");
    }

    cmd.status_checked()?
}
