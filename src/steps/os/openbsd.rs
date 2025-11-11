use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::execution_context::RunType;
use crate::terminal::print_separator;
use color_eyre::eyre::Result;
use rust_i18n::t;
use std::fs;

fn is_openbsd_current(ctx: &ExecutionContext) -> Result<bool> {
    let motd_content = fs::read_to_string("/etc/motd")?;
    let is_current = ["-current", "-beta"].iter().any(|&s| motd_content.contains(s));
    match ctx.run_type() {
        RunType::Dry | RunType::Damp => {
            println!("{}", t!("Checking if /etc/motd contains -current or -beta"));
        }
        RunType::Wet => {}
    }
    Ok(is_current)
}

pub fn upgrade_openbsd(ctx: &ExecutionContext) -> Result<()> {
    print_separator(t!("OpenBSD Update"));

    let sudo = ctx.require_sudo()?;

    let is_current = is_openbsd_current(ctx)?;

    match ctx.run_type() {
        RunType::Dry | RunType::Damp => {
            println!("{}", t!("Would upgrade the OpenBSD system"));
            return Ok(());
        }
        RunType::Wet => {}
    }

    if is_current {
        sudo.execute(ctx, "/usr/sbin/sysupgrade")?.arg("-sn").status_checked()
    } else {
        sudo.execute(ctx, "/usr/sbin/syspatch")?.status_checked()
    }
}

pub fn upgrade_packages(ctx: &ExecutionContext) -> Result<()> {
    print_separator(t!("OpenBSD Packages"));

    let sudo = ctx.require_sudo()?;

    let is_current = is_openbsd_current(ctx)?;

    if ctx.config().cleanup() {
        sudo.execute(ctx, "/usr/sbin/pkg_delete")?.arg("-ac").status_checked()?;
    }

    let mut command = sudo.execute(ctx, "/usr/sbin/pkg_add")?;
    command.arg("-u");
    if is_current {
        command.arg("-Dsnap");
    }
    command.status_checked()
}
