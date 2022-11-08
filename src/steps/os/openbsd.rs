use crate::executor::RunType;
use crate::terminal::print_separator;
use crate::utils::require_option;
use anyhow::Result;
use std::path::PathBuf;

pub fn upgrade_openbsd(sudo: Option<&PathBuf>, run_type: RunType) -> Result<()> {
    let sudo = require_option(sudo, String::from("No sudo detected"))?;
    print_separator("OpenBSD Update");
    run_type
        .execute(sudo)
        .args(&["/usr/sbin/sysupgrade", "-n"])
        .status_checked()
}

pub fn upgrade_packages(sudo: Option<&PathBuf>, run_type: RunType) -> Result<()> {
    let sudo = require_option(sudo, String::from("No sudo detected"))?;
    print_separator("OpenBSD Packages");
    run_type
        .execute(sudo)
        .args(&["/usr/sbin/pkg_add", "-u"])
        .status_checked()
}
