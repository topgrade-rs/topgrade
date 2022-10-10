use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use ini::Ini;
use log::{debug, warn};

use crate::error::{SkipStep, TopgradeError};
use crate::execution_context::ExecutionContext;
use crate::executor::{CommandExt, RunType};
use crate::steps::os::archlinux;
use crate::terminal::{print_separator, print_warning};
use crate::utils::{require, require_option, which, PathExt};
use crate::Step;

static OS_RELEASE_PATH: &str = "/etc/os-release";

#[allow(clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Distribution {
    Alpine,
    Arch,
    Bedrock,
    CentOS,
    ClearLinux,
    Fedora,
    Debian,
    Gentoo,
    Suse,
    Void,
    Solus,
    Exherbo,
    NixOS,
    KDENeon,
}

impl Distribution {
    fn parse_os_release(os_release: &ini::Ini) -> Result<Self> {
        let section = os_release.general_section();
        let id = section.get("ID");
        let id_like: Option<Vec<&str>> = section.get("ID_LIKE").map(|s| s.split_whitespace().collect());

        Ok(match id {
            Some("alpine") => Distribution::Alpine,
            Some("centos") | Some("rhel") | Some("ol") => Distribution::CentOS,
            Some("clear-linux-os") => Distribution::ClearLinux,
            Some("fedora") => Distribution::Fedora,
            Some("void") => Distribution::Void,
            Some("debian") | Some("pureos") => Distribution::Debian,
            Some("arch") | Some("anarchy") | Some("manjaro-arm") | Some("garuda") | Some("artix") => Distribution::Arch,
            Some("solus") => Distribution::Solus,
            Some("gentoo") => Distribution::Gentoo,
            Some("exherbo") => Distribution::Exherbo,
            Some("nixos") => Distribution::NixOS,
            Some("neon") => Distribution::KDENeon,
            _ => {
                if let Some(id_like) = id_like {
                    if id_like.contains(&"debian") || id_like.contains(&"ubuntu") {
                        return Ok(Distribution::Debian);
                    } else if id_like.contains(&"centos") {
                        return Ok(Distribution::CentOS);
                    } else if id_like.contains(&"suse") {
                        return Ok(Distribution::Suse);
                    } else if id_like.contains(&"arch") || id_like.contains(&"archlinux") {
                        return Ok(Distribution::Arch);
                    } else if id_like.contains(&"alpine") {
                        return Ok(Distribution::Alpine);
                    } else if id_like.contains(&"fedora") {
                        return Ok(Distribution::Fedora);
                    }
                }
                return Err(TopgradeError::UnknownLinuxDistribution.into());
            }
        })
    }

    pub fn detect() -> Result<Self> {
        if PathBuf::from("/bedrock").exists() {
            return Ok(Distribution::Bedrock);
        }

        if PathBuf::from(OS_RELEASE_PATH).exists() {
            let os_release = Ini::load_from_file(OS_RELEASE_PATH)?;

            return Self::parse_os_release(&os_release);
        }

        Err(TopgradeError::UnknownLinuxDistribution.into())
    }

    pub fn upgrade(self, ctx: &ExecutionContext) -> Result<()> {
        print_separator("System update");

        match self {
            Distribution::Alpine => upgrade_alpine_linux(ctx),
            Distribution::Arch => archlinux::upgrade_arch_linux(ctx),
            Distribution::CentOS | Distribution::Fedora => upgrade_redhat(ctx),
            Distribution::ClearLinux => upgrade_clearlinux(ctx),
            Distribution::Debian => upgrade_debian(ctx),
            Distribution::Gentoo => upgrade_gentoo(ctx),
            Distribution::Suse => upgrade_suse(ctx),
            Distribution::Void => upgrade_void(ctx),
            Distribution::Solus => upgrade_solus(ctx),
            Distribution::Exherbo => upgrade_exherbo(ctx),
            Distribution::NixOS => upgrade_nixos(ctx),
            Distribution::KDENeon => upgrade_neon(ctx),
            Distribution::Bedrock => update_bedrock(ctx),
        }
    }

    pub fn show_summary(self) {
        if let Distribution::Arch = self {
            archlinux::show_pacnew();
        }
    }

    pub fn redhat_based(self) -> bool {
        matches!(self, Distribution::CentOS | Distribution::Fedora)
    }
}

fn update_bedrock(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), String::from("Sudo required"))?;

    ctx.run_type().execute(sudo).args(&["brl", "update"]);

    let output = Command::new("brl").arg("list").output()?;
    debug!("brl list: {:?} {:?}", output.stdout, output.stderr);

    let parsed_output = String::from_utf8(output.stdout).unwrap();
    for distribution in parsed_output.trim().split('\n') {
        debug!("Bedrock distribution {}", distribution);
        match distribution {
            "arch" => archlinux::upgrade_arch_linux(ctx)?,
            "debian" | "ubuntu" => upgrade_debian(ctx)?,
            "centos" | "fedora" => upgrade_redhat(ctx)?,
            "bedrock" => upgrade_bedrock_strata(ctx)?,
            _ => {
                warn!("Unknown distribution {}", distribution);
            }
        }
    }

    Ok(())
}

fn is_wsl() -> Result<bool> {
    let output = Command::new("uname").arg("-r").check_output()?;
    debug!("Uname output: {}", output);
    Ok(output.contains("microsoft"))
}

fn upgrade_alpine_linux(ctx: &ExecutionContext) -> Result<()> {
    let apk = require("apk")?;
    let sudo = ctx.sudo().as_ref().unwrap();

    ctx.run_type().execute(sudo).arg(&apk).arg("update").check_run()?;
    ctx.run_type().execute(sudo).arg(&apk).arg("upgrade").check_run()
}

fn upgrade_redhat(ctx: &ExecutionContext) -> Result<()> {
    if let Some(ostree) = which("rpm-ostree") {
        if ctx.config().rpm_ostree() {
            let mut command = ctx.run_type().execute(ostree);
            command.arg("upgrade");
            return command.check_run();
        }
    };

    if let Some(sudo) = &ctx.sudo() {
        let mut command = ctx.run_type().execute(&sudo);
        command
            .arg(which("dnf").unwrap_or_else(|| Path::new("yum").to_path_buf()))
            .arg(if ctx.config().redhat_distro_sync() {
                "distro-sync"
            } else {
                "upgrade"
            });

        if let Some(args) = ctx.config().dnf_arguments() {
            command.args(args.split_whitespace());
        }

        if ctx.config().yes(Step::System) {
            command.arg("-y");
        }

        command.check_run()?;
    } else {
        print_warning("No sudo detected. Skipping system upgrade");
    }

    Ok(())
}

fn upgrade_bedrock_strata(ctx: &ExecutionContext) -> Result<()> {
    if let Some(sudo) = ctx.sudo() {
        ctx.run_type().execute(&sudo).args(&["brl", "update"]).check_run()?;
    } else {
        print_warning("No sudo detected. Skipping system upgrade");
    }

    Ok(())
}

fn upgrade_suse(ctx: &ExecutionContext) -> Result<()> {
    if let Some(sudo) = ctx.sudo() {
        ctx.run_type().execute(&sudo).args(&["zypper", "refresh"]).check_run()?;

        ctx.run_type()
            .execute(&sudo)
            .args(&["zypper", "dist-upgrade"])
            .check_run()?;
    } else {
        print_warning("No sudo detected. Skipping system upgrade");
    }

    Ok(())
}

fn upgrade_void(ctx: &ExecutionContext) -> Result<()> {
    if let Some(sudo) = ctx.sudo() {
        let mut command = ctx.run_type().execute(&sudo);
        command.args(&["xbps-install", "-Su", "xbps"]);
        if ctx.config().yes(Step::System) {
            command.arg("-y");
        }
        command.check_run()?;

        let mut command = ctx.run_type().execute(&sudo);
        command.args(&["xbps-install", "-u"]);
        if ctx.config().yes(Step::System) {
            command.arg("-y");
        }
        command.check_run()?;
    } else {
        print_warning("No sudo detected. Skipping system upgrade");
    }

    Ok(())
}

fn upgrade_gentoo(ctx: &ExecutionContext) -> Result<()> {
    let run_type = ctx.run_type();

    if let Some(sudo) = &ctx.sudo() {
        if let Some(layman) = which("layman") {
            run_type.execute(&sudo).arg(layman).args(&["-s", "ALL"]).check_run()?;
        }

        println!("Syncing portage");
        run_type
            .execute(&sudo)
            .args(&["emerge", "--sync"])
            .args(
                ctx.config()
                    .emerge_sync_flags()
                    .map(|s| s.split_whitespace().collect())
                    .unwrap_or_else(|| vec!["-q"]),
            )
            .check_run()?;

        if let Some(eix_update) = which("eix-update") {
            run_type.execute(&sudo).arg(eix_update).check_run()?;
        }

        run_type
            .execute(&sudo)
            .arg("emerge")
            .args(
                ctx.config()
                    .emerge_update_flags()
                    .map(|s| s.split_whitespace().collect())
                    .unwrap_or_else(|| vec!["-uDNa", "--with-bdeps=y", "world"]),
            )
            .check_run()?;
    } else {
        print_warning("No sudo detected. Skipping system upgrade");
    }

    Ok(())
}

fn upgrade_debian(ctx: &ExecutionContext) -> Result<()> {
    if let Some(sudo) = &ctx.sudo() {
        let apt = which("apt-fast")
            .or_else(|| which("nala"))
            .unwrap_or_else(|| PathBuf::from("apt-get"));

        let is_nala = apt.ends_with("nala");
        if !is_nala {
            ctx.run_type().execute(&sudo).arg(&apt).arg("update").check_run()?;
        }

        let mut command = ctx.run_type().execute(&sudo);
        command.arg(&apt);
        if is_nala {
            command.arg("upgrade");
        } else {
            command.arg("dist-upgrade");
        };
        if ctx.config().yes(Step::System) {
            command.arg("-y");
        }
        if let Some(args) = ctx.config().apt_arguments() {
            command.args(args.split_whitespace());
        }
        command.check_run()?;

        if ctx.config().cleanup() {
            ctx.run_type().execute(&sudo).arg(&apt).arg("clean").check_run()?;

            let mut command = ctx.run_type().execute(&sudo);
            command.arg(&apt).arg("autoremove");
            if ctx.config().yes(Step::System) {
                command.arg("-y");
            }
            command.check_run()?;
        }
    } else {
        print_warning("No sudo detected. Skipping system upgrade");
    }

    Ok(())
}

pub fn run_deb_get(ctx: &ExecutionContext) -> Result<()> {
    let deb_get = require("deb-get")?;

    print_separator("deb-get");

    ctx.execute_elevated(&deb_get, false)?.arg("upgrade").check_run()?;

    if ctx.config().cleanup() {
        ctx.execute_elevated(&deb_get, false)?.arg("clean").check_run()?;
    }

    Ok(())
}

fn upgrade_solus(ctx: &ExecutionContext) -> Result<()> {
    if let Some(sudo) = ctx.sudo() {
        ctx.run_type().execute(&sudo).args(&["eopkg", "upgrade"]).check_run()?;
    } else {
        print_warning("No sudo detected. Skipping system upgrade");
    }

    Ok(())
}

pub fn run_pacdef(ctx: &ExecutionContext) -> Result<()> {
    let pacdef = require("pacdef")?;

    print_separator("pacdef");

    ctx.run_type().execute(&pacdef).arg("sync").check_run()?;

    println!();
    ctx.run_type().execute(&pacdef).arg("review").check_run()
}

pub fn run_pacstall(ctx: &ExecutionContext) -> Result<()> {
    let pacstall = require("pacstall")?;

    print_separator("Pacstall");

    ctx.run_type().execute(&pacstall).arg("-U").check_run()?;
    ctx.run_type().execute(pacstall).arg("-Up").check_run()
}

fn upgrade_clearlinux(ctx: &ExecutionContext) -> Result<()> {
    if let Some(sudo) = &ctx.sudo() {
        ctx.run_type().execute(&sudo).args(&["swupd", "update"]).check_run()?;
    } else {
        print_warning("No sudo detected. Skipping system upgrade");
    }

    Ok(())
}

fn upgrade_exherbo(ctx: &ExecutionContext) -> Result<()> {
    if let Some(sudo) = ctx.sudo() {
        ctx.run_type().execute(&sudo).args(&["cave", "sync"]).check_run()?;

        ctx.run_type()
            .execute(&sudo)
            .args(&["cave", "resolve", "world", "-c1", "-Cs", "-km", "-Km", "-x"])
            .check_run()?;

        if ctx.config().cleanup() {
            ctx.run_type()
                .execute(&sudo)
                .args(&["cave", "purge", "-x"])
                .check_run()?;
        }

        ctx.run_type()
            .execute(&sudo)
            .args(&["cave", "fix-linkage", "-x", "--", "-Cs"])
            .check_run()?;

        ctx.run_type()
            .execute(&sudo)
            .args(&["eclectic", "config", "interactive"])
            .check_run()?;
    } else {
        print_warning("No sudo detected. Skipping system upgrade");
    }

    Ok(())
}

fn upgrade_nixos(ctx: &ExecutionContext) -> Result<()> {
    if let Some(sudo) = ctx.sudo() {
        ctx.run_type()
            .execute(&sudo)
            .args(&["/run/current-system/sw/bin/nixos-rebuild", "switch", "--upgrade"])
            .check_run()?;

        if ctx.config().cleanup() {
            ctx.run_type()
                .execute(&sudo)
                .args(&["/run/current-system/sw/bin/nix-collect-garbage", "-d"])
                .check_run()?;
        }
    } else {
        print_warning("No sudo detected. Skipping system upgrade");
    }

    Ok(())
}

fn upgrade_neon(ctx: &ExecutionContext) -> Result<()> {
    // KDE neon is ubuntu based but uses it's own manager, pkcon
    // running apt update with KDE neon is an error
    // in theory rpm based distributions use pkcon as well, though that
    // seems rare
    // if that comes up we need to create a Distribution::PackageKit or some such
    if let Some(sudo) = &ctx.sudo() {
        let pkcon = which("pkcon").unwrap();
        // pkcon ignores update with update and refresh provided together
        ctx.run_type().execute(&sudo).arg(&pkcon).arg("refresh").check_run()?;
        let mut exe = ctx.run_type().execute(&sudo);
        let cmd = exe.arg(&pkcon).arg("update");
        if ctx.config().yes(Step::System) {
            cmd.arg("-y");
        }
        if ctx.config().cleanup() {
            cmd.arg("--autoremove");
        }
        // from pkcon man, exit code 5 is 'Nothing useful was done.'
        cmd.check_run_with_codes(&[5])?;
    }

    Ok(())
}

pub fn run_needrestart(sudo: Option<&PathBuf>, run_type: RunType) -> Result<()> {
    let sudo = require_option(sudo, String::from("sudo is not installed"))?;
    let needrestart = require("needrestart")?;
    let distribution = Distribution::detect()?;

    if distribution.redhat_based() {
        return Err(SkipStep(String::from("needrestart will be ran by the package manager")).into());
    }

    print_separator("Check for needed restarts");

    run_type.execute(&sudo).arg(needrestart).check_run()?;

    Ok(())
}

pub fn run_fwupdmgr(ctx: &ExecutionContext) -> Result<()> {
    let fwupdmgr = require("fwupdmgr")?;

    if is_wsl()? {
        return Err(SkipStep(String::from("Should not run in WSL")).into());
    }

    print_separator("Firmware upgrades");

    ctx.run_type()
        .execute(&fwupdmgr)
        .arg("refresh")
        .check_run_with_codes(&[2])?;

    let mut updmgr = ctx.run_type().execute(&fwupdmgr);

    if ctx.config().firmware_upgrade() {
        updmgr.arg("update");
        if ctx.config().yes(Step::System) {
            updmgr.arg("-y");
        }
    } else {
        updmgr.arg("get-updates");
    }
    updmgr.check_run_with_codes(&[2])
}

pub fn flatpak_update(ctx: &ExecutionContext) -> Result<()> {
    let flatpak = require("flatpak")?;
    let sudo = require_option(ctx.sudo().as_ref(), String::from("sudo is not installed"))?;
    let cleanup = ctx.config().cleanup();
    let run_type = ctx.run_type();
    print_separator("Flatpak User Packages");

    run_type
        .execute(&flatpak)
        .args(&["update", "--user", "-y"])
        .check_run()?;
    if cleanup {
        run_type
            .execute(&flatpak)
            .args(&["uninstall", "--user", "--unused"])
            .check_run()?;
    }

    print_separator("Flatpak System Packages");
    if ctx.config().flatpak_use_sudo() || std::env::var("SSH_CLIENT").is_ok() {
        run_type
            .execute(&sudo)
            .arg(&flatpak)
            .args(&["update", "--system", "-y"])
            .check_run()?;
        if cleanup {
            run_type
                .execute(sudo)
                .arg(flatpak)
                .args(&["uninstall", "--system", "--unused"])
                .check_run()?;
        }
    } else {
        run_type
            .execute(&flatpak)
            .args(&["update", "--system", "-y"])
            .check_run()?;
        if cleanup {
            run_type
                .execute(flatpak)
                .args(&["uninstall", "--system", "--unused"])
                .check_run()?;
        }
    }

    Ok(())
}

pub fn run_snap(sudo: Option<&PathBuf>, run_type: RunType) -> Result<()> {
    let sudo = require_option(sudo, String::from("sudo is not installed"))?;
    let snap = require("snap")?;

    if !PathBuf::from("/var/snapd.socket").exists() && !PathBuf::from("/run/snapd.socket").exists() {
        return Err(SkipStep(String::from("Snapd socket does not exist")).into());
    }
    print_separator("snap");

    run_type.execute(sudo).arg(snap).arg("refresh").check_run()
}

pub fn run_pihole_update(sudo: Option<&PathBuf>, run_type: RunType) -> Result<()> {
    let sudo = require_option(sudo, String::from("sudo is not installed"))?;
    let pihole = require("pihole")?;
    Path::new("/opt/pihole/update.sh").require()?;

    print_separator("pihole");

    run_type.execute(sudo).arg(pihole).arg("-up").check_run()
}

pub fn run_config_update(ctx: &ExecutionContext) -> Result<()> {
    let sudo = require_option(ctx.sudo().as_ref(), String::from("sudo is not installed"))?;
    if ctx.config().yes(Step::ConfigUpdate) {
        return Err(SkipStep("Skipped in --yes".to_string()).into());
    }

    if let Ok(etc_update) = require("etc-update") {
        print_separator("Configuration update");
        ctx.run_type().execute(sudo).arg(etc_update).check_run()?;
    } else if let Ok(pacdiff) = require("pacdiff") {
        if std::env::var("DIFFPROG").is_err() {
            require("vim")?;
        }

        print_separator("Configuration update");
        ctx.execute_elevated(&pacdiff, false)?.check_run()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_template(os_release_file: &str, expected_distribution: Distribution) {
        let os_release = Ini::load_from_str(os_release_file).unwrap();
        assert_eq!(
            Distribution::parse_os_release(&os_release).unwrap(),
            expected_distribution
        );
    }

    #[test]
    fn test_arch_linux() {
        test_template(include_str!("os_release/arch"), Distribution::Arch);
        test_template(include_str!("os_release/arch32"), Distribution::Arch);
    }

    #[test]
    fn test_centos() {
        test_template(include_str!("os_release/centos"), Distribution::CentOS);
    }

    #[test]
    fn test_rhel() {
        test_template(include_str!("os_release/rhel"), Distribution::CentOS);
    }

    #[test]
    fn test_clearlinux() {
        test_template(include_str!("os_release/clearlinux"), Distribution::ClearLinux);
    }

    #[test]
    fn test_debian() {
        test_template(include_str!("os_release/debian"), Distribution::Debian);
    }

    #[test]
    fn test_ubuntu() {
        test_template(include_str!("os_release/ubuntu"), Distribution::Debian);
    }

    #[test]
    fn test_mint() {
        test_template(include_str!("os_release/mint"), Distribution::Debian);
    }

    #[test]
    fn test_opensuse() {
        test_template(include_str!("os_release/opensuse"), Distribution::Suse);
    }

    #[test]
    fn test_oraclelinux() {
        test_template(include_str!("os_release/oracle"), Distribution::CentOS);
    }

    #[test]
    fn test_fedora() {
        test_template(include_str!("os_release/fedora"), Distribution::Fedora);
    }

    #[test]
    fn test_antergos() {
        test_template(include_str!("os_release/antergos"), Distribution::Arch);
    }

    #[test]
    fn test_manjaro() {
        test_template(include_str!("os_release/manjaro"), Distribution::Arch);
    }

    #[test]
    fn test_manjaro_arm() {
        test_template(include_str!("os_release/manjaro-arm"), Distribution::Arch);
    }

    #[test]
    fn test_anarchy() {
        test_template(include_str!("os_release/anarchy"), Distribution::Arch);
    }

    #[test]
    fn test_gentoo() {
        test_template(include_str!("os_release/gentoo"), Distribution::Gentoo);
    }

    #[test]
    fn test_exherbo() {
        test_template(include_str!("os_release/exherbo"), Distribution::Exherbo);
    }

    #[test]
    fn test_amazon_linux() {
        test_template(include_str!("os_release/amazon_linux"), Distribution::CentOS);
    }

    #[test]
    fn test_nixos() {
        test_template(include_str!("os_release/nixos"), Distribution::NixOS);
    }

    #[test]
    fn test_fedoraremixonwsl() {
        test_template(include_str!("os_release/fedoraremixforwsl"), Distribution::Fedora);
    }

    #[test]
    fn test_pengwinonwsl() {
        test_template(include_str!("os_release/pengwinonwsl"), Distribution::Debian);
    }

    #[test]
    fn test_artix() {
        test_template(include_str!("os_release/artix"), Distribution::Arch);
    }

    #[test]
    fn test_garuda() {
        test_template(include_str!("os_release/garuda"), Distribution::Arch);
    }

    #[test]
    fn test_pureos() {
        test_template(include_str!("os_release/pureos"), Distribution::Debian);
    }
}
