use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::Result;
use ini::Ini;
use rust_i18n::t;
use tracing::{debug, warn};

use crate::command::CommandExt;
use crate::error::{SkipStep, TopgradeError};
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::steps::generic::is_wsl;
use crate::steps::os::archlinux;
use crate::sudo::SudoExecuteOpts;
use crate::terminal::{print_separator, prompt_yesno};
use crate::utils::{require, require_one, which, PathExt};
use crate::HOME_DIR;

static OS_RELEASE_PATH: &str = "/etc/os-release";

#[allow(clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Distribution {
    Alpine,
    AOSC,
    Wolfi,
    Arch,
    Bedrock,
    CentOS,
    Chimera,
    ClearLinux,
    Fedora,
    FedoraImmutable,
    Debian,
    Gentoo,
    NILRT,
    OpenMandriva,
    OpenSuseTumbleweed,
    PCLinuxOS,
    Suse,
    SuseMicro,
    Vanilla,
    Void,
    Solus,
    Exherbo,
    NixOS,
    KDENeon,
    Nobara,
}

impl Distribution {
    fn parse_os_release(os_release: &Ini) -> Result<Self> {
        let section = os_release.general_section();
        let id = section.get("ID");
        let name = section.get("NAME");
        let variant = section.get("VARIANT");
        let id_like: Option<Vec<&str>> = section.get("ID_LIKE").map(|s| s.split_whitespace().collect());

        Ok(match id {
            Some("alpine") => Distribution::Alpine,
            Some("aosc") => Distribution::AOSC,
            Some("chimera") => Distribution::Chimera,
            Some("wolfi") => Distribution::Wolfi,
            Some("centos") | Some("rhel") | Some("ol") => Distribution::CentOS,
            Some("clear-linux-os") => Distribution::ClearLinux,
            Some("fedora") => Distribution::match_fedora_variant(&variant),
            Some("nilrt") => Distribution::NILRT,
            Some("nobara") => Distribution::Nobara,
            Some("void") => Distribution::Void,
            Some("debian") | Some("pureos") | Some("Deepin") | Some("linuxmint") => Distribution::Debian,
            Some("arch") | Some("manjaro-arm") | Some("garuda") | Some("artix") | Some("cachyos") => Distribution::Arch,
            Some("solus") => Distribution::Solus,
            Some("gentoo") | Some("funtoo") => Distribution::Gentoo,
            Some("exherbo") => Distribution::Exherbo,
            Some("nixos") => Distribution::NixOS,
            Some("opensuse-microos") => Distribution::SuseMicro,
            Some("neon") => Distribution::KDENeon,
            Some("openmandriva") => Distribution::OpenMandriva,
            Some("pclinuxos") => Distribution::PCLinuxOS,
            _ => {
                if let Some(name) = name {
                    if name.contains("Vanilla") {
                        return Ok(Distribution::Vanilla);
                    }
                }
                if let Some(id_like) = id_like {
                    if id_like.contains(&"debian") || id_like.contains(&"ubuntu") {
                        return Ok(Distribution::Debian);
                    } else if id_like.contains(&"centos") {
                        return Ok(Distribution::CentOS);
                    } else if id_like.contains(&"suse") {
                        let id_variant = id.unwrap_or_default();
                        return if id_variant.contains("tumbleweed") {
                            Ok(Distribution::OpenSuseTumbleweed)
                        } else {
                            Ok(Distribution::Suse)
                        };
                    } else if id_like.contains(&"arch") || id_like.contains(&"archlinux") {
                        return Ok(Distribution::Arch);
                    } else if id_like.contains(&"alpine") {
                        return Ok(Distribution::Alpine);
                    } else if id_like.contains(&"fedora") {
                        return Ok(Distribution::match_fedora_variant(&variant));
                    }
                }
                return Err(TopgradeError::UnknownLinuxDistribution.into());
            }
        })
    }

    fn match_fedora_variant(variant: &Option<&str>) -> Self {
        if let Some("Silverblue" | "Kinoite" | "Sericea" | "Onyx" | "IoT Edition" | "Sway Atomic" | "CoreOS") = variant
        {
            Distribution::FedoraImmutable
        } else {
            Distribution::Fedora
        }
    }

    pub fn detect() -> Result<Self> {
        if PathBuf::from("/bedrock").exists() {
            return Ok(Distribution::Bedrock);
        }

        if PathBuf::from(OS_RELEASE_PATH).exists() {
            let os_release = Ini::load_from_file(OS_RELEASE_PATH)?;

            if os_release.general_section().is_empty() {
                return Err(TopgradeError::EmptyOSReleaseFile.into());
            }

            return Self::parse_os_release(&os_release);
        }

        Err(TopgradeError::EmptyOSReleaseFile.into())
    }

    pub fn upgrade(self, ctx: &ExecutionContext) -> Result<()> {
        print_separator(t!("System update"));

        match self {
            Distribution::Alpine => upgrade_alpine_linux(ctx),
            Distribution::Chimera => upgrade_chimera_linux(ctx),
            Distribution::Wolfi => upgrade_wolfi_linux(ctx),
            Distribution::Arch => archlinux::upgrade_arch_linux(ctx),
            Distribution::CentOS | Distribution::Fedora => upgrade_redhat(ctx),
            Distribution::FedoraImmutable => upgrade_fedora_immutable(ctx),
            Distribution::ClearLinux => upgrade_clearlinux(ctx),
            Distribution::Debian => upgrade_debian(ctx),
            Distribution::Gentoo => upgrade_gentoo(ctx),
            Distribution::Suse => upgrade_suse(ctx),
            Distribution::SuseMicro => upgrade_suse_micro(ctx),
            Distribution::OpenSuseTumbleweed => upgrade_opensuse_tumbleweed(ctx),
            Distribution::Vanilla => upgrade_vanilla(ctx),
            Distribution::Void => upgrade_void(ctx),
            Distribution::Solus => upgrade_solus(ctx),
            Distribution::Exherbo => upgrade_exherbo(ctx),
            Distribution::NixOS => upgrade_nixos(ctx),
            Distribution::KDENeon => upgrade_neon(ctx),
            Distribution::Bedrock => update_bedrock(ctx),
            Distribution::OpenMandriva => upgrade_openmandriva(ctx),
            Distribution::PCLinuxOS => upgrade_pclinuxos(ctx),
            Distribution::Nobara => upgrade_nobara(ctx),
            Distribution::NILRT => upgrade_nilrt(ctx),
            Distribution::AOSC => upgrade_aosc(ctx),
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
    let brl = require("brl")?;

    let output = Command::new(&brl).arg("list").output_checked_utf8()?;
    debug!("brl list: {:?} {:?}", output.stdout, output.stderr);

    for distribution in output.stdout.trim().lines() {
        debug!("Bedrock distribution {}", distribution);
        match distribution {
            "arch" => archlinux::upgrade_arch_linux(ctx)?,
            "debian" | "ubuntu" | "linuxmint" => upgrade_debian(ctx)?,
            "centos" | "fedora" => upgrade_redhat(ctx)?,
            "bedrock" => upgrade_bedrock_strata(ctx)?,
            _ => {
                warn!("Unknown distribution {}", distribution);
            }
        }
    }

    Ok(())
}

fn upgrade_aosc(ctx: &ExecutionContext) -> Result<()> {
    let oma = require("oma")?;
    let sudo = ctx.require_sudo()?;

    let mut cmd = sudo.execute(ctx, &oma)?;
    cmd.arg("upgrade");

    if ctx.config().yes(Step::System) {
        cmd.arg("-y");
    }

    cmd.status_checked()
}

fn upgrade_alpine_linux(ctx: &ExecutionContext) -> Result<()> {
    let apk = require("apk")?;
    let sudo = ctx.require_sudo()?;

    sudo.execute(ctx, &apk)?.arg("update").status_checked()?;
    sudo.execute(ctx, &apk)?.arg("upgrade").status_checked()
}

fn upgrade_chimera_linux(ctx: &ExecutionContext) -> Result<()> {
    let apk = require("apk")?;
    let sudo = ctx.require_sudo()?;

    sudo.execute(ctx, &apk)?.arg("update").status_checked()?;
    sudo.execute(ctx, &apk)?.arg("upgrade").status_checked()
}

fn upgrade_wolfi_linux(ctx: &ExecutionContext) -> Result<()> {
    let apk = require("apk")?;
    let sudo = ctx.require_sudo()?;

    sudo.execute(ctx, &apk)?.arg("update").status_checked()?;
    sudo.execute(ctx, &apk)?.arg("upgrade").status_checked()
}

fn upgrade_redhat(ctx: &ExecutionContext) -> Result<()> {
    if let Some(bootc) = which("bootc") {
        if ctx.config().bootc() {
            let sudo = ctx.require_sudo()?;
            return sudo.execute(ctx, &bootc)?.arg("upgrade").status_checked();
        }
    }

    if let Some(ostree) = which("rpm-ostree") {
        if ctx.config().rpm_ostree() {
            let mut command = ctx.execute(ostree);
            command.arg("upgrade");
            return command.status_checked();
        }
    };

    let dnf = require_one(["dnf", "yum"])?;
    let sudo = ctx.require_sudo()?;

    let mut command = sudo.execute(ctx, &dnf)?;
    command.arg(if ctx.config().redhat_distro_sync() {
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

    command.status_checked()?;
    Ok(())
}

fn upgrade_nobara(ctx: &ExecutionContext) -> Result<()> {
    let dnf = require("dnf")?;
    let sudo = ctx.require_sudo()?;

    let mut update_command = sudo.execute(ctx, &dnf)?;

    if ctx.config().yes(Step::System) {
        update_command.arg("-y");
    }

    update_command.arg("update");
    // See https://nobaraproject.org/docs/upgrade-troubleshooting/how-do-i-update-the-system/
    update_command.args([
        "rpmfusion-nonfree-release",
        "rpmfusion-free-release",
        "fedora-repos",
        "nobara-repos",
    ]);
    update_command.arg("--refresh").status_checked()?;

    let mut upgrade_command = sudo.execute(ctx, &dnf)?;

    if ctx.config().yes(Step::System) {
        upgrade_command.arg("-y");
    }

    upgrade_command.arg("distro-sync");

    upgrade_command.status_checked()?;
    Ok(())
}

fn upgrade_nilrt(ctx: &ExecutionContext) -> Result<()> {
    let opkg = require("opkg")?;
    let sudo = ctx.require_sudo()?;

    sudo.execute(ctx, &opkg)?.arg("update").status_checked()?;
    sudo.execute(ctx, &opkg)?.arg("upgrade").status_checked()
}

fn upgrade_fedora_immutable(ctx: &ExecutionContext) -> Result<()> {
    if let Some(bootc) = which("bootc") {
        if ctx.config().bootc() {
            let sudo = ctx.require_sudo()?;
            return sudo.execute(ctx, &bootc)?.arg("upgrade").status_checked();
        }
    }

    let ostree = require("rpm-ostree")?;
    let mut command = ctx.execute(ostree);
    command.arg("upgrade");
    command.status_checked()?;
    Ok(())
}

fn upgrade_bedrock_strata(ctx: &ExecutionContext) -> Result<()> {
    let brl = require("brl")?;
    let sudo = ctx.require_sudo()?;

    sudo.execute(ctx, &brl)?.arg("update").status_checked()?;

    Ok(())
}

fn upgrade_suse(ctx: &ExecutionContext) -> Result<()> {
    let zypper = require("zypper")?;
    let sudo = ctx.require_sudo()?;

    sudo.execute(ctx, &zypper)?.arg("refresh").status_checked()?;

    let mut cmd = sudo.execute(ctx, &zypper)?;
    cmd.arg(if ctx.config().suse_dup() {
        "dist-upgrade"
    } else {
        "update"
    });
    if ctx.config().yes(Step::System) {
        cmd.arg("-y");
    }

    cmd.status_checked()?;

    Ok(())
}

fn upgrade_opensuse_tumbleweed(ctx: &ExecutionContext) -> Result<()> {
    let zypper = require("zypper")?;
    let sudo = ctx.require_sudo()?;

    sudo.execute(ctx, &zypper)?.arg("refresh").status_checked()?;

    let mut cmd = sudo.execute(ctx, &zypper)?;
    cmd.arg("dist-upgrade");
    if ctx.config().yes(Step::System) {
        cmd.arg("-y");
    }

    cmd.status_checked()?;

    Ok(())
}

fn upgrade_suse_micro(ctx: &ExecutionContext) -> Result<()> {
    let upd = require("transactional-update")?;
    let sudo = ctx.require_sudo()?;

    let mut cmd = sudo.execute(ctx, &upd)?;
    if ctx.config().yes(Step::System) {
        cmd.arg("-n");
    }

    cmd.arg("dup").status_checked()?;

    Ok(())
}

fn upgrade_openmandriva(ctx: &ExecutionContext) -> Result<()> {
    let dnf = require("dnf")?;
    let sudo = ctx.require_sudo()?;

    let mut command = sudo.execute(ctx, &dnf)?;

    command.arg("upgrade");

    if let Some(args) = ctx.config().dnf_arguments() {
        command.args(args.split_whitespace());
    }

    if ctx.config().yes(Step::System) {
        command.arg("-y");
    }

    command.status_checked()?;

    Ok(())
}

fn upgrade_pclinuxos(ctx: &ExecutionContext) -> Result<()> {
    let apt_get = require("apt-get")?;
    let sudo = ctx.require_sudo()?;

    let mut command_update = sudo.execute(ctx, &apt_get)?;

    command_update.arg("update");

    if let Some(args) = ctx.config().dnf_arguments() {
        command_update.args(args.split_whitespace());
    }

    if ctx.config().yes(Step::System) {
        command_update.arg("-y");
    }

    command_update.status_checked()?;

    let mut cmd = sudo.execute(ctx, &apt_get)?;
    cmd.arg("dist-upgrade");
    if ctx.config().yes(Step::System) {
        cmd.arg("-y");
    }
    cmd.status_checked()?;

    Ok(())
}

fn upgrade_vanilla(ctx: &ExecutionContext) -> Result<()> {
    let apx = require("apx")?;

    let mut update = ctx.execute(&apx);
    update.args(["update", "--all"]);
    if ctx.config().yes(Step::System) {
        update.arg("-y");
    }
    update.status_checked()?;

    let mut upgrade = ctx.execute(&apx);
    update.args(["upgrade", "--all"]);
    if ctx.config().yes(Step::System) {
        upgrade.arg("-y");
    }
    upgrade.status_checked()?;

    Ok(())
}

fn upgrade_void(ctx: &ExecutionContext) -> Result<()> {
    let xbps = require("xbps-install")?;
    let sudo = ctx.require_sudo()?;

    let mut command = sudo.execute(ctx, &xbps)?;
    command.args(["-Su", "xbps"]);
    if ctx.config().yes(Step::System) {
        command.arg("-y");
    }
    command.status_checked()?;

    let mut command = sudo.execute(ctx, &xbps)?;
    command.arg("-u");
    if ctx.config().yes(Step::System) {
        command.arg("-y");
    }
    command.status_checked()?;

    Ok(())
}

fn upgrade_gentoo(ctx: &ExecutionContext) -> Result<()> {
    let emerge = require("emerge")?;
    let sudo = ctx.require_sudo()?;

    if let Some(layman) = which("layman") {
        sudo.execute(ctx, &layman)?.args(["-s", "ALL"]).status_checked()?;
    }

    println!("{}", t!("Syncing portage"));
    if let Some(ego) = which("ego") {
        // The Funtoo team doesn't recommend running both ego sync and emerge --sync
        sudo.execute(ctx, &ego)?.arg("sync").status_checked()?;
    } else {
        sudo.execute(ctx, &emerge)?
            .arg("--sync")
            .args(
                ctx.config()
                    .emerge_sync_flags()
                    .map(|s| s.split_whitespace().collect())
                    .unwrap_or_else(|| vec!["-q"]),
            )
            .status_checked()?;
    }

    if let Some(eix_update) = which("eix-update") {
        sudo.execute(ctx, &eix_update)?.status_checked()?;
    }

    sudo.execute(ctx, &emerge)?
        .args(
            ctx.config()
                .emerge_update_flags()
                .map(|s| s.split_whitespace().collect())
                .unwrap_or_else(|| vec!["-uDNa", "--with-bdeps=y", "world"]),
        )
        .status_checked()?;

    Ok(())
}

enum AptKind {
    AptFast,
    Mist,
    Nala,
    AptGet,
}

fn detect_apt() -> Result<(AptKind, PathBuf)> {
    use AptKind::*;

    if let Some(apt_fast) = which("apt-fast") {
        Ok((AptFast, apt_fast))
    } else if let Some(mist) = which("mist") {
        Ok((Mist, mist))
    } else if Path::new("/usr/bin/nala").exists() {
        Ok((Nala, Path::new("/usr/bin/nala").to_path_buf()))
    } else {
        Ok((AptGet, require("apt-get")?))
    }
}

fn upgrade_debian(ctx: &ExecutionContext) -> Result<()> {
    use AptKind::*;

    let (kind, apt) = detect_apt()?;

    // MIST does not require `sudo`
    if matches!(kind, Mist) {
        ctx.execute(&apt).arg("update").status_checked()?;
        ctx.execute(&apt).arg("upgrade").status_checked()?;

        // Simply return as MIST does not have `clean` and `autoremove`
        // subcommands, neither the `-y` option (for now maybe?).
        return Ok(());
    }

    let sudo = ctx.require_sudo()?;
    if !matches!(kind, Nala) {
        sudo.execute(ctx, &apt)?
            .arg("update")
            .status_checked_with_codes(&[0, 100])?;
    }

    let mut command = sudo.execute(ctx, &apt)?;
    if matches!(kind, Nala) {
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
    command.status_checked()?;

    if ctx.config().cleanup() {
        sudo.execute(ctx, &apt)?.arg("clean").status_checked()?;

        let mut command = sudo.execute(ctx, &apt)?;
        command.arg("autoremove");
        if ctx.config().yes(Step::System) {
            command.arg("-y");
        }
        command.status_checked()?;
    }

    Ok(())
}

pub fn run_deb_get(ctx: &ExecutionContext) -> Result<()> {
    let deb_get = require("deb-get")?;

    print_separator("deb-get");

    ctx.execute(&deb_get).arg("update").status_checked()?;
    ctx.execute(&deb_get)
        .arg("upgrade")
        // Since the `apt` step already updates all other apt packages, don't check for updates
        //  to all packages here. This does suboptimally check for updates for deb-get packages
        //  that apt can update (that were installed via a repository), but that is only a few,
        //  and there's nothing we can do about that.
        .arg("--dg-only")
        .status_checked()?;

    if ctx.config().cleanup() {
        let output = ctx.execute(&deb_get).arg("clean").output_checked()?;
        // Swallow the output, as it's very noisy and not useful.
        //  The output is automatically printed as part of `output_checked` when an error occurs.
        println!("{}", t!("<output from `deb-get clean` omitted>"));
        debug!("`deb-get clean` output: {output:?}");
    }

    Ok(())
}

fn upgrade_solus(ctx: &ExecutionContext) -> Result<()> {
    let eopkg = require("eopkg")?;
    let sudo = ctx.require_sudo()?;

    let mut cmd = sudo.execute(ctx, &eopkg)?;
    if ctx.config().yes(Step::System) {
        cmd.arg("-y");
    }
    cmd.arg("upgrade").status_checked()?;

    Ok(())
}

pub fn run_am(ctx: &ExecutionContext) -> Result<()> {
    let am = require("am")?;

    print_separator("AM");

    let mut am = ctx.execute(am);

    if ctx.config().yes(Step::AM) {
        am.arg("-U");
    } else {
        am.arg("-u");
    }

    am.status_checked()
}

pub fn run_appman(ctx: &ExecutionContext) -> Result<()> {
    let appman = require("appman")?;

    print_separator("appman");

    ctx.execute(appman).arg("-u").status_checked()
}

pub fn run_pacdef(ctx: &ExecutionContext) -> Result<()> {
    let pacdef = require("pacdef")?;

    print_separator("pacdef");

    let output = ctx.execute(&pacdef).arg("version").output_checked()?;
    let string = String::from_utf8(output.stdout)?;
    let new_version = string.contains("version: 1");

    if new_version {
        let mut cmd = ctx.execute(&pacdef);
        cmd.args(["package", "sync"]);
        if ctx.config().yes(Step::System) {
            cmd.arg("--noconfirm");
        }
        cmd.status_checked()?;

        println!();
        ctx.execute(&pacdef).args(["package", "review"]).status_checked()?;
    } else {
        let mut cmd = ctx.execute(&pacdef);
        cmd.arg("sync");
        if ctx.config().yes(Step::System) {
            cmd.arg("--noconfirm");
        }

        cmd.status_checked()?;

        println!();
        ctx.execute(&pacdef).arg("review").status_checked()?;
    }
    Ok(())
}

pub fn run_pacstall(ctx: &ExecutionContext) -> Result<()> {
    let pacstall = require("pacstall")?;

    print_separator("Pacstall");

    let mut update_cmd = ctx.execute(&pacstall);
    let mut upgrade_cmd = ctx.execute(pacstall);

    if ctx.config().yes(Step::Pacstall) {
        update_cmd.arg("-P");
        upgrade_cmd.arg("-P");
    }

    update_cmd.arg("-U").status_checked()?;
    upgrade_cmd.arg("-Up").status_checked()
}

pub fn run_pkgfile(ctx: &ExecutionContext) -> Result<()> {
    let pkgfile = require("pkgfile")?;

    if !ctx.config().enable_pkgfile() {
        return Err(SkipStep("Pkgfile isn't enabled".to_string()).into());
    }

    print_separator("pkgfile");

    let sudo = ctx.require_sudo()?;
    sudo.execute(ctx, pkgfile)?.arg("--update").status_checked()
}

pub fn run_mandb(ctx: &ExecutionContext) -> Result<()> {
    let mandb = require("mandb")?;

    if !ctx.config().enable_mandb() {
        return Err(SkipStep(t!("ManDB isn't enabled").to_string()).into());
    }

    print_separator(t!("System Manuals"));

    let sudo = ctx.require_sudo()?;
    sudo.execute(ctx, &mandb)?.status_checked()?;

    print_separator(t!("User Manuals"));

    ctx.execute(&mandb).arg("--user-db").status_checked()
}

pub fn run_packer_nu(ctx: &ExecutionContext) -> Result<()> {
    let nu = require("nu")?;
    let packer_home = HOME_DIR.join(".local/share/nushell/packer");

    packer_home.clone().require()?;

    print_separator("packer.nu");

    ctx.execute(nu)
        .env("PWD", "/")
        .env("NU_PACKER_HOME", packer_home)
        .args([
            "-c",
            "use ~/.local/share/nushell/packer/start/packer.nu/api_layer/packer.nu; packer update",
        ])
        .status_checked()
}

fn upgrade_clearlinux(ctx: &ExecutionContext) -> Result<()> {
    let swupd = require("swupd")?;
    let sudo = ctx.require_sudo()?;

    let mut cmd = sudo.execute(ctx, &swupd)?;
    cmd.arg("update");
    if ctx.config().yes(Step::System) {
        cmd.arg("--assume=yes");
    }
    cmd.status_checked()?;

    Ok(())
}

fn upgrade_exherbo(ctx: &ExecutionContext) -> Result<()> {
    let cave = require("cave")?;
    let eclectic = require("eclectic")?;
    let sudo = ctx.require_sudo()?;

    sudo.execute(ctx, &cave)?.arg("sync").status_checked()?;

    sudo.execute(ctx, &cave)?
        .args(["resolve", "world", "-c1", "-Cs", "-km", "-Km", "-x"])
        .status_checked()?;

    if ctx.config().cleanup() {
        sudo.execute(ctx, &cave)?.args(["purge", "-x"]).status_checked()?;
    }

    sudo.execute(ctx, &cave)?
        .args(["fix-linkage", "-x", "--", "-Cs"])
        .status_checked()?;

    sudo.execute(ctx, &eclectic)?
        .args(["config", "interactive"])
        .status_checked()?;

    Ok(())
}

fn upgrade_nixos(ctx: &ExecutionContext) -> Result<()> {
    let sudo = ctx.require_sudo()?;

    let mut command = sudo.execute(ctx, "/run/current-system/sw/bin/nixos-rebuild")?;
    command.args(["switch", "--upgrade"]);

    if let Some(args) = ctx.config().nix_arguments() {
        command.args(args.split_whitespace());
    }
    command.status_checked()?;

    if ctx.config().cleanup() {
        sudo.execute(ctx, "/run/current-system/sw/bin/nix-collect-garbage")?
            .arg("-d")
            .status_checked()?;
    }

    Ok(())
}

fn upgrade_neon(ctx: &ExecutionContext) -> Result<()> {
    // KDE neon is ubuntu based but uses it's own manager, pkcon
    // running apt update with KDE neon is an error
    // in theory rpm based distributions use pkcon as well, though that
    // seems rare
    // if that comes up we need to create a Distribution::PackageKit or some such

    let pkcon = require("pkcon")?;
    let sudo = ctx.require_sudo()?;

    // pkcon ignores update with update and refresh provided together
    sudo.execute(ctx, &pkcon)?.arg("refresh").status_checked()?;

    let mut exe = sudo.execute(ctx, &pkcon)?;
    let cmd = exe.arg("update");
    if ctx.config().yes(Step::System) {
        cmd.arg("-y");
    }
    if ctx.config().cleanup() {
        cmd.arg("--autoremove");
    }
    // from pkcon man, exit code 5 is 'Nothing useful was done.'
    cmd.status_checked_with_codes(&[5])?;

    Ok(())
}

/// `needrestart` should be skipped if:
///
/// 1. This is a redhat-based distribution
/// 2. This is a debian-based distribution and it is using `nala` as the `apt`
///    alternative
fn should_skip_needrestart() -> Result<()> {
    let distribution = Distribution::detect()?;
    let msg = t!("needrestart will be ran by the package manager");

    if distribution.redhat_based() {
        return Err(SkipStep(String::from(msg)).into());
    }

    if matches!(distribution, Distribution::Debian) {
        let (apt_kind, _) = detect_apt()?;
        if matches!(apt_kind, AptKind::Nala) {
            return Err(SkipStep(String::from(msg)).into());
        }
    }

    Ok(())
}

pub fn run_needrestart(ctx: &ExecutionContext) -> Result<()> {
    let needrestart = require("needrestart")?;

    should_skip_needrestart()?;

    print_separator(t!("Check for needed restarts"));

    let sudo = ctx.require_sudo()?;
    sudo.execute(ctx, &needrestart)?.status_checked()?;

    Ok(())
}

pub fn run_fwupdmgr(ctx: &ExecutionContext) -> Result<()> {
    let fwupdmgr = require("fwupdmgr")?;

    if is_wsl()? {
        return Err(SkipStep(t!("Should not run in WSL").to_string()).into());
    }

    print_separator(t!("Firmware upgrades"));

    ctx.execute(&fwupdmgr).arg("refresh").status_checked_with_codes(&[2])?;

    let mut updmgr = ctx.execute(&fwupdmgr);

    if ctx.config().firmware_upgrade() {
        updmgr.arg("update");
        if ctx.config().yes(Step::System) {
            updmgr.arg("-y");
        }
    } else {
        updmgr.arg("get-updates");
    }
    updmgr.status_checked_with_codes(&[2])
}

pub fn run_flatpak(ctx: &ExecutionContext) -> Result<()> {
    let flatpak = require("flatpak")?;

    let cleanup = ctx.config().cleanup();
    let yes = ctx.config().yes(Step::Flatpak);
    print_separator("Flatpak User Packages");

    let mut update_args = vec!["update", "--user"];
    if yes {
        update_args.push("-y");
    }
    ctx.execute(&flatpak).args(&update_args).status_checked()?;

    if cleanup {
        let mut cleanup_args = vec!["uninstall", "--user", "--unused"];
        if yes {
            cleanup_args.push("-y");
        }
        ctx.execute(&flatpak).args(&cleanup_args).status_checked()?;
    }

    print_separator(t!("Flatpak System Packages"));
    if ctx.config().flatpak_use_sudo() || std::env::var("SSH_CLIENT").is_ok() {
        let sudo = ctx.require_sudo()?;
        let mut update_args = vec!["update", "--system"];
        if yes {
            update_args.push("-y");
        }
        sudo.execute(ctx, &flatpak)?.args(&update_args).status_checked()?;
        if cleanup {
            let mut cleanup_args = vec!["uninstall", "--system", "--unused"];
            if yes {
                cleanup_args.push("-y");
            }
            sudo.execute(ctx, &flatpak)?.args(&cleanup_args).status_checked()?;
        }
    } else {
        let mut update_args = vec!["update", "--system"];
        if yes {
            update_args.push("-y");
        }
        ctx.execute(&flatpak).args(&update_args).status_checked()?;
        if cleanup {
            let mut cleanup_args = vec!["uninstall", "--system", "--unused"];
            if yes {
                cleanup_args.push("-y");
            }
            ctx.execute(flatpak).args(&cleanup_args).status_checked()?;
        }
    }

    Ok(())
}

pub fn run_snap(ctx: &ExecutionContext) -> Result<()> {
    let snap = require("snap")?;

    if !PathBuf::from("/var/snapd.socket").exists() && !PathBuf::from("/run/snapd.socket").exists() {
        return Err(SkipStep(t!("Snapd socket does not exist").to_string()).into());
    }
    print_separator("snap");

    let sudo = ctx.require_sudo()?;
    sudo.execute(ctx, &snap)?.arg("refresh").status_checked()
}

pub fn run_pihole_update(ctx: &ExecutionContext) -> Result<()> {
    let pihole = require("pihole")?;
    Path::new("/opt/pihole/update.sh").require()?;

    print_separator("pihole");

    let sudo = ctx.require_sudo()?;
    sudo.execute(ctx, &pihole)?.arg("-up").status_checked()
}

pub fn run_protonup_update(ctx: &ExecutionContext) -> Result<()> {
    let protonup = require("protonup")?;

    print_separator("protonup");

    let mut cmd = ctx.execute(protonup);
    if ctx.config().yes(Step::Protonup) {
        cmd.arg("--yes");
    }
    cmd.status_checked()?;

    Ok(())
}

pub fn run_distrobox_update(ctx: &ExecutionContext) -> Result<()> {
    let distrobox = require("distrobox")?;

    print_separator("Distrobox");
    match (
        match (
            ctx.execute(distrobox).arg("upgrade"),
            ctx.config().distrobox_containers(),
        ) {
            (r, Some(c)) => {
                if c.is_empty() {
                    return Err(SkipStep(t!("You need to specify at least one container").to_string()).into());
                }
                r.args(c)
            }
            (r, None) => r.arg("--all"),
        },
        ctx.config().distrobox_root(),
    ) {
        (r, true) => r.arg("--root"),
        (r, false) => r,
    }
    .status_checked()
}

pub fn run_dkp_pacman_update(ctx: &ExecutionContext) -> Result<()> {
    let dkp_pacman = require("dkp-pacman")?;

    print_separator("Devkitpro pacman");

    let sudo = ctx.require_sudo()?;

    sudo.execute(ctx, &dkp_pacman)?.arg("-Syu").status_checked()?;

    if ctx.config().cleanup() {
        sudo.execute(ctx, &dkp_pacman)?.arg("-Scc").status_checked()?;
    }

    Ok(())
}

pub fn run_config_update(ctx: &ExecutionContext) -> Result<()> {
    // The `config_update` step always requests user input, so when running with `--yes` we need to skip the step entirely
    if ctx.config().yes(Step::ConfigUpdate) {
        return Err(SkipStep(t!("Skipped in --yes").to_string()).into());
    }

    if let Ok(etc_update) = require("etc-update") {
        print_separator(t!("Configuration update"));
        let sudo = ctx.require_sudo()?;
        sudo.execute(ctx, etc_update)?.status_checked()?;
    } else if let Ok(pacdiff) = require("pacdiff") {
        // When `DIFFPROG` is unset, `pacdiff` uses `vim` by default
        if std::env::var("DIFFPROG").is_err() {
            require("vim")?;
        }

        print_separator(t!("Configuration update"));
        let sudo = ctx.require_sudo()?;
        sudo.execute_opts(ctx, &pacdiff, SudoExecuteOpts::new().preserve_env_list(&["DIFFPROG"]))?
            .status_checked()?;
    }

    Ok(())
}

pub fn run_lure_update(ctx: &ExecutionContext) -> Result<()> {
    let lure = require("lure")?;

    print_separator("LURE");

    let mut exe = ctx.execute(lure);

    if ctx.config().yes(Step::Lure) {
        exe.args(["-i=false", "up"]);
    } else {
        exe.arg("up");
    }

    exe.status_checked()
}

pub fn run_waydroid(ctx: &ExecutionContext) -> Result<()> {
    let waydroid = require("waydroid")?;

    let status = ctx.execute(&waydroid).arg("status").output_checked_utf8()?;
    // example output of `waydroid status`:
    //
    // ```sh
    // $ waydroid status
    // Session:        RUNNING
    // Container:      RUNNING
    // Vendor type:    MAINLINE
    // IP address:     192.168.240.112
    // Session user:   w568w(1000)
    // Wayland display:        wayland-0
    // ```
    //
    // ```sh
    // $ waydroid status
    // Session:        STOPPED
    // Vendor type:    MAINLINE
    // ```
    let session = status
        .stdout
        .lines()
        .find(|line| line.contains("Session:"))
        .unwrap_or_else(|| panic!("the output of `waydroid status` should contain `Session:`"));
    let is_container_running = session.contains("RUNNING");
    let assume_yes = ctx.config().yes(Step::Waydroid);

    print_separator("Waydroid");

    if is_container_running && !assume_yes {
        let update_allowed = prompt_yesno(&t!(
            "Going to execute `waydroid upgrade`, which would STOP the running container, is this ok?"
        ))?;
        if !update_allowed {
            return Err(
                SkipStep(t!("Skip the Waydroid step because the user don't want to proceed").to_string()).into(),
            );
        }
    }

    let sudo = ctx.require_sudo()?;
    sudo.execute(ctx, &waydroid)?.arg("upgrade").status_checked()
}

pub fn run_auto_cpufreq(ctx: &ExecutionContext) -> Result<()> {
    let auto_cpu_freq = require("auto-cpufreq")?;
    if auto_cpu_freq != PathBuf::from("/usr/local/bin/auto-cpufreq") {
        return Err(SkipStep(String::from(
            "`auto-cpufreq` was not installed by the official installer, but presumably by a package manager.",
        ))
        .into());
    }

    print_separator("auto-cpufreq");

    let sudo = ctx.require_sudo()?;
    sudo.execute(ctx, &auto_cpu_freq)?.arg("--update").status_checked()
}

pub fn run_cinnamon_spices_updater(ctx: &ExecutionContext) -> Result<()> {
    let cinnamon_spice_updater = require("cinnamon-spice-updater")?;

    print_separator("Cinnamon spices");

    ctx.execute(cinnamon_spice_updater).arg("--update-all").status_checked()
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
    fn test_wolfi() {
        test_template(include_str!("os_release/wolfi"), Distribution::Wolfi);
    }

    #[test]
    fn test_arch_linux() {
        test_template(include_str!("os_release/arch"), Distribution::Arch);
        test_template(include_str!("os_release/arch32"), Distribution::Arch);
    }

    #[test]
    fn test_aosc() {
        test_template(include_str!("os_release/aosc"), Distribution::AOSC);
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
    fn test_fedora_immutable() {
        test_template(
            include_str!("os_release/fedorasilverblue"),
            Distribution::FedoraImmutable,
        );
        test_template(include_str!("os_release/fedorakinoite"), Distribution::FedoraImmutable);
        test_template(include_str!("os_release/fedoraonyx"), Distribution::FedoraImmutable);
        test_template(include_str!("os_release/fedorasericea"), Distribution::FedoraImmutable);
        test_template(include_str!("os_release/fedoraiot"), Distribution::FedoraImmutable);
        test_template(
            include_str!("os_release/fedoraswayatomic"),
            Distribution::FedoraImmutable,
        );
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
    fn test_gentoo() {
        test_template(include_str!("os_release/gentoo"), Distribution::Gentoo);
    }

    #[test]
    fn test_funtoo() {
        test_template(include_str!("os_release/funtoo"), Distribution::Gentoo);
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

    #[test]
    fn test_deepin() {
        test_template(include_str!("os_release/deepin"), Distribution::Debian);
    }

    #[test]
    fn test_vanilla() {
        test_template(include_str!("os_release/vanilla"), Distribution::Vanilla);
    }

    #[test]
    fn test_solus() {
        test_template(include_str!("os_release/solus"), Distribution::Solus);
    }

    #[test]
    fn test_nobara() {
        test_template(include_str!("os_release/nobara"), Distribution::Nobara);
    }

    #[test]
    fn test_nilrt() {
        test_template(include_str!("os_release/nilrt"), Distribution::NILRT);
    }

    #[test]
    fn test_coreos() {
        test_template(include_str!("os_release/coreos"), Distribution::FedoraImmutable);
    }

    #[test]
    fn test_aurora() {
        test_template(include_str!("os_release/aurora"), Distribution::FedoraImmutable);
    }

    #[test]
    fn test_bluefin() {
        test_template(include_str!("os_release/bluefin"), Distribution::FedoraImmutable);
    }

    #[test]
    fn test_bazzite() {
        test_template(include_str!("os_release/bazzite"), Distribution::FedoraImmutable);
    }

    #[test]
    fn test_cachyos() {
        test_template(include_str!("os_release/cachyos"), Distribution::Arch);
    }
}
