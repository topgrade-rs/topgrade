use std::path::{Path, PathBuf};

use color_eyre::eyre;
use color_eyre::eyre::{Context, Result};
use rust_i18n::t;
use walkdir::WalkDir;

use crate::command::CommandExt;
use crate::error::TopgradeError;
use crate::execution_context::ExecutionContext;
use crate::step::Step;
use crate::utils::which;
use crate::{config, output_changed_message};

pub trait ArchPackageManager {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()>;
}

pub struct YayParu {
    executable: PathBuf,
    pacman: PathBuf,
}

impl ArchPackageManager for YayParu {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        if ctx.config().show_arch_news() {
            ctx.execute(&self.executable)
                .arg("-Pw")
                .status_checked_with_codes(&[1, 0])?;
        }

        ctx.execute(&self.executable)
            .arg("--pacman")
            .arg(&self.pacman)
            .arg("-Syu")
            .args(ctx.config().yay_arguments().split_whitespace())
            .arg_if(ctx.config().yes(Step::System), "--noconfirm")
            .status_checked()?;

        if ctx.config().cleanup() {
            ctx.execute(&self.executable)
                .arg("--pacman")
                .arg(&self.pacman)
                .arg("-Scc")
                .arg_if(ctx.config().yes(Step::System), "--noconfirm")
                .status_checked()?;
        }

        Ok(())
    }
}

impl YayParu {
    fn get(exec_name: &str, pacman: &Path) -> Option<Self> {
        Some(Self {
            executable: which(exec_name)?,
            pacman: pacman.to_owned(),
        })
    }
}

pub struct GarudaUpdate {
    executable: PathBuf,
}

impl ArchPackageManager for GarudaUpdate {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        ctx.execute(&self.executable)
            .env("UPDATE_AUR", "1")
            .env("SKIP_MIRRORLIST", "1")
            .env_if(ctx.config().yes(Step::System), "PACMAN_NOCONFIRM", "1")
            .args(ctx.config().garuda_update_arguments().split_whitespace())
            .status_checked()?;

        Ok(())
    }
}

impl GarudaUpdate {
    fn get() -> Option<Self> {
        Some(Self {
            executable: which("garuda-update")?,
        })
    }
}

pub struct Trizen {
    executable: PathBuf,
}

impl ArchPackageManager for Trizen {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        ctx.execute(&self.executable)
            .arg("-Syu")
            .args(ctx.config().trizen_arguments().split_whitespace())
            .arg_if(ctx.config().yes(Step::System), "--noconfirm")
            .status_checked()?;

        if ctx.config().cleanup() {
            ctx.execute(&self.executable)
                .arg("-Sc")
                .arg_if(ctx.config().yes(Step::System), "--noconfirm")
                .status_checked()?;
        }

        Ok(())
    }
}

impl Trizen {
    fn get() -> Option<Self> {
        Some(Self {
            executable: which("trizen")?,
        })
    }
}

pub struct Pacman {
    executable: PathBuf,
}

impl ArchPackageManager for Pacman {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        let sudo = ctx.require_sudo()?;
        sudo.execute(ctx, &self.executable)?
            .arg("-Syu")
            .arg_if(ctx.config().yes(Step::System), "--noconfirm")
            .status_checked()?;

        if ctx.config().cleanup() {
            sudo.execute(ctx, &self.executable)?
                .arg("-Scc")
                .arg_if(ctx.config().yes(Step::System), "--noconfirm")
                .status_checked()?;
        }

        Ok(())
    }
}

impl Pacman {
    pub fn get() -> Option<Self> {
        Some(Self {
            executable: which("powerpill").unwrap_or_else(|| PathBuf::from("pacman")),
        })
    }
}

pub struct Pikaur {
    executable: PathBuf,
}

impl Pikaur {
    fn get() -> Option<Self> {
        Some(Self {
            executable: which("pikaur")?,
        })
    }
}

impl ArchPackageManager for Pikaur {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        ctx.execute(&self.executable)
            .arg("-Syu")
            .args(ctx.config().pikaur_arguments().split_whitespace())
            .arg_if(ctx.config().yes(Step::System), "--noconfirm")
            .status_checked()?;

        if ctx.config().cleanup() {
            ctx.execute(&self.executable)
                .arg("-Sc")
                .arg_if(ctx.config().yes(Step::System), "--noconfirm")
                .status_checked()?;
        }

        Ok(())
    }
}

pub struct Pamac {
    executable: PathBuf,
}

impl Pamac {
    fn get() -> Option<Self> {
        Some(Self {
            executable: which("pamac")?,
        })
    }
}
impl ArchPackageManager for Pamac {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        ctx.execute(&self.executable)
            .arg("upgrade")
            .args(ctx.config().pamac_arguments().split_whitespace())
            .arg_if(ctx.config().yes(Step::System), "--no-confirm")
            .status_checked()?;

        if ctx.config().cleanup() {
            ctx.execute(&self.executable)
                .arg("clean")
                .arg_if(ctx.config().yes(Step::System), "--no-confirm")
                .status_checked()?;
        }

        Ok(())
    }
}

pub struct Aura {
    executable: PathBuf,
}

impl Aura {
    fn get() -> Option<Self> {
        Some(Self {
            executable: which("aura")?,
        })
    }
}

impl ArchPackageManager for Aura {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        use semver::Version;

        let version_cmd_output = ctx
            .execute(&self.executable)
            .always()
            .arg("--version")
            .output_checked_utf8()?;
        // Output will be something like: "aura x.x.x\n"
        let version_cmd_stdout = version_cmd_output.stdout;
        let version_str = version_cmd_stdout.trim_start_matches("aura ").trim_end();
        let version = Version::parse(version_str)
            .wrap_err_with(|| output_changed_message!("aura --version", "invalid version"))?;

        // Aura, since version 4.0.6, no longer needs sudo.
        //
        // https://github.com/fosskers/aura/releases/tag/v4.0.6
        let version_no_sudo = Version::new(4, 0, 6);

        if version >= version_no_sudo {
            ctx.execute(&self.executable)
                .arg("-Au")
                .args(ctx.config().aura_aur_arguments().split_whitespace())
                .arg_if(ctx.config().yes(Step::System), "--noconfirm")
                .status_checked()?;

            ctx.execute(&self.executable)
                .arg("-Syu")
                .args(ctx.config().aura_pacman_arguments().split_whitespace())
                .arg_if(ctx.config().yes(Step::System), "--noconfirm")
                .status_checked()?;
        } else {
            let sudo = ctx.require_sudo()?;

            sudo.execute(ctx, &self.executable)?
                .arg("-Au")
                .args(ctx.config().aura_aur_arguments().split_whitespace())
                .arg_if(ctx.config().yes(Step::System), "--noconfirm")
                .status_checked()?;

            sudo.execute(ctx, &self.executable)?
                .arg("-Syu")
                .args(ctx.config().aura_pacman_arguments().split_whitespace())
                .arg_if(ctx.config().yes(Step::System), "--noconfirm")
                .status_checked()?;
        }

        Ok(())
    }
}

pub struct Shelly {
    executable: PathBuf,
}

impl Shelly {
    fn get() -> Option<Self> {
        Some(Self {
            executable: which("shelly")?,
        })
    }
}

impl ArchPackageManager for Shelly {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        if ctx.config().show_arch_news() {
            ctx.execute(&self.executable)
                .arg("news")
                .arg_if(ctx.config().yes(Step::System), "--no-confirm")
                .status_checked()?;
        }

        ctx.execute(&self.executable)
            .arg("sync")
            .arg_if(ctx.config().yes(Step::System), "--no-confirm")
            .status_checked()?;

        ctx.execute(&self.executable)
            .arg("upgrade-all")
            .args(ctx.config().shelly_arguments().split_whitespace())
            .arg_if(ctx.config().should_run(Step::Flatpak), "--no-flatpak")
            .arg_if(ctx.config().yes(Step::System), "--no-confirm")
            .status_checked()?;

        Ok(())
    }
}

fn box_package_manager<P: 'static + ArchPackageManager>(package_manager: P) -> Box<dyn ArchPackageManager> {
    Box::new(package_manager) as Box<dyn ArchPackageManager>
}

pub fn get_arch_package_manager(ctx: &ExecutionContext) -> Option<Box<dyn ArchPackageManager>> {
    let pacman = which("powerpill").unwrap_or_else(|| PathBuf::from("pacman"));

    match ctx.config().arch_package_manager() {
        config::ArchPackageManager::Autodetect => GarudaUpdate::get()
            .map(box_package_manager)
            .or_else(|| YayParu::get("paru", &pacman).map(box_package_manager))
            .or_else(|| YayParu::get("yay", &pacman).map(box_package_manager))
            .or_else(|| Trizen::get().map(box_package_manager))
            .or_else(|| Pikaur::get().map(box_package_manager))
            .or_else(|| Pamac::get().map(box_package_manager))
            .or_else(|| Pacman::get().map(box_package_manager))
            .or_else(|| Aura::get().map(box_package_manager)),
        config::ArchPackageManager::GarudaUpdate => GarudaUpdate::get().map(box_package_manager),
        config::ArchPackageManager::Trizen => Trizen::get().map(box_package_manager),
        config::ArchPackageManager::Paru => YayParu::get("paru", &pacman).map(box_package_manager),
        config::ArchPackageManager::Yay => YayParu::get("yay", &pacman).map(box_package_manager),
        config::ArchPackageManager::Pacman => Pacman::get().map(box_package_manager),
        config::ArchPackageManager::Pikaur => Pikaur::get().map(box_package_manager),
        config::ArchPackageManager::Pamac => Pamac::get().map(box_package_manager),
        config::ArchPackageManager::Aura => Aura::get().map(box_package_manager),
        config::ArchPackageManager::Shelly => Shelly::get().map(box_package_manager),
    }
}

pub fn upgrade_arch_linux(ctx: &ExecutionContext) -> Result<()> {
    let package_manager =
        get_arch_package_manager(ctx).ok_or_else(|| eyre::Report::from(TopgradeError::FailedGettingPackageManager))?;
    package_manager.upgrade(ctx)
}

pub fn show_pacnew() {
    let mut iter = WalkDir::new("/etc")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|f| {
            f.path()
                .extension()
                .filter(|ext| ext == &"pacnew" || ext == &"pacsave")
                .is_some()
        })
        .peekable();

    if iter.peek().is_some() {
        println!("\n{}", t!("Pacman backup configuration files found:"));

        for entry in iter {
            println!("{}", entry.path().display());
        }
    }
}
