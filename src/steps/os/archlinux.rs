use std::env::var_os;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre;
use color_eyre::eyre::Result;
use walkdir::WalkDir;

use crate::command::CommandExt;
use crate::error::TopgradeError;
use crate::execution_context::ExecutionContext;
use crate::sudo::Sudo;
use crate::utils::which;
use crate::{config, Step};

fn get_execution_path() -> OsString {
    let mut path = OsString::from("/usr/bin:");
    path.push(var_os("PATH").unwrap());
    path
}

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
            Command::new(&self.executable).arg("-Pw").status_checked()?;
        }

        let mut command = ctx.run_type().execute(&self.executable);

        command
            .arg("--pacman")
            .arg(&self.pacman)
            .arg("-Syu")
            .args(ctx.config().yay_arguments().split_whitespace())
            .env("PATH", get_execution_path());

        if ctx.config().yes(Step::System) {
            command.arg("--noconfirm");
        }
        command.status_checked()?;

        if ctx.config().cleanup() {
            let mut command = ctx.run_type().execute(&self.executable);
            command.arg("--pacman").arg(&self.pacman).arg("-Scc");
            if ctx.config().yes(Step::System) {
                command.arg("--noconfirm");
            }
            command.status_checked()?;
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

pub struct Trizen {
    executable: PathBuf,
}

impl ArchPackageManager for Trizen {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        let mut command = ctx.run_type().execute(&self.executable);

        command
            .arg("-Syu")
            .args(ctx.config().trizen_arguments().split_whitespace())
            .env("PATH", get_execution_path());

        if ctx.config().yes(Step::System) {
            command.arg("--noconfirm");
        }
        command.status_checked()?;

        if ctx.config().cleanup() {
            let mut command = ctx.run_type().execute(&self.executable);
            command.arg("-Sc");
            if ctx.config().yes(Step::System) {
                command.arg("--noconfirm");
            }
            command.status_checked()?;
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
    sudo: Sudo,
    executable: PathBuf,
}

impl ArchPackageManager for Pacman {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        let mut command = ctx.run_type().execute(&self.sudo);
        command
            .arg(&self.executable)
            .arg("-Syu")
            .env("PATH", get_execution_path());
        if ctx.config().yes(Step::System) {
            command.arg("--noconfirm");
        }
        command.status_checked()?;

        if ctx.config().cleanup() {
            let mut command = ctx.run_type().execute(&self.sudo);
            command.arg(&self.executable).arg("-Scc");
            if ctx.config().yes(Step::System) {
                command.arg("--noconfirm");
            }
            command.status_checked()?;
        }

        Ok(())
    }
}

impl Pacman {
    pub fn get(ctx: &ExecutionContext) -> Option<Self> {
        Some(Self {
            executable: which("powerpill").unwrap_or_else(|| PathBuf::from("pacman")),
            sudo: ctx.sudo().to_owned()?,
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
        let mut command = ctx.run_type().execute(&self.executable);

        command
            .arg("-Syu")
            .args(ctx.config().pikaur_arguments().split_whitespace())
            .env("PATH", get_execution_path());

        if ctx.config().yes(Step::System) {
            command.arg("--noconfirm");
        }

        command.status_checked()?;

        if ctx.config().cleanup() {
            let mut command = ctx.run_type().execute(&self.executable);
            command.arg("-Sc");
            if ctx.config().yes(Step::System) {
                command.arg("--noconfirm");
            }
            command.status_checked()?;
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
        let mut command = ctx.run_type().execute(&self.executable);

        command
            .arg("upgrade")
            .args(ctx.config().pamac_arguments().split_whitespace())
            .env("PATH", get_execution_path());

        if ctx.config().yes(Step::System) {
            command.arg("--no-confirm");
        }

        command.status_checked()?;

        if ctx.config().cleanup() {
            let mut command = ctx.run_type().execute(&self.executable);
            command.arg("clean");
            if ctx.config().yes(Step::System) {
                command.arg("--no-confirm");
            }
            command.status_checked()?;
        }

        Ok(())
    }
}

pub struct Aura {
    executable: PathBuf,
    sudo: PathBuf,
}

impl Aura {
    fn get(ctx: &ExecutionContext) -> Option<Self> {
        Some(Self {
            executable: which("aura")?,
            sudo: ctx.sudo().to_owned()?,
        })
    }
}

impl ArchPackageManager for Aura {
    fn upgrade(&self, ctx: &ExecutionContext) -> Result<()> {
        let sudo = which("sudo").unwrap_or_else(PathBuf::new);
        let mut aur_update = ctx.run_type().execute(&sudo);

        if sudo.ends_with("sudo") {
            aur_update
                .arg(&self.executable)
                .arg("-Au")
                .args(ctx.config().aura_aur_arguments().split_whitespace());
            if ctx.config().yes(Step::System) {
                aur_update.arg("--noconfirm");
            }

            aur_update.status_checked()?;
        } else {
            println!("Aura requires sudo installed to work with AUR packages")
        }

        let mut pacman_update = ctx.run_type().execute(&self.sudo);
        pacman_update
            .arg(&self.executable)
            .arg("-Syu")
            .args(ctx.config().aura_pacman_arguments().split_whitespace());
        if ctx.config().yes(Step::System) {
            pacman_update.arg("--noconfirm");
        }
        pacman_update.status_checked()?;

        Ok(())
    }
}

fn box_package_manager<P: 'static + ArchPackageManager>(package_manager: P) -> Box<dyn ArchPackageManager> {
    Box::new(package_manager) as Box<dyn ArchPackageManager>
}

pub fn get_arch_package_manager(ctx: &ExecutionContext) -> Option<Box<dyn ArchPackageManager>> {
    let pacman = which("powerpill").unwrap_or_else(|| PathBuf::from("pacman"));

    match ctx.config().arch_package_manager() {
        config::ArchPackageManager::Autodetect => YayParu::get("paru", &pacman)
            .map(box_package_manager)
            .or_else(|| YayParu::get("yay", &pacman).map(box_package_manager))
            .or_else(|| Trizen::get().map(box_package_manager))
            .or_else(|| Pikaur::get().map(box_package_manager))
            .or_else(|| Pamac::get().map(box_package_manager))
            .or_else(|| Pacman::get(ctx).map(box_package_manager))
            .or_else(|| Aura::get(ctx).map(box_package_manager)),
        config::ArchPackageManager::Trizen => Trizen::get().map(box_package_manager),
        config::ArchPackageManager::Paru => YayParu::get("paru", &pacman).map(box_package_manager),
        config::ArchPackageManager::Yay => YayParu::get("yay", &pacman).map(box_package_manager),
        config::ArchPackageManager::Pacman => Pacman::get(ctx).map(box_package_manager),
        config::ArchPackageManager::Pikaur => Pikaur::get().map(box_package_manager),
        config::ArchPackageManager::Pamac => Pamac::get().map(box_package_manager),
        config::ArchPackageManager::Aura => Aura::get(ctx).map(box_package_manager),
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
        println!("\nPacman backup configuration files found:");

        for entry in iter {
            println!("{}", entry.path().display());
        }
    }
}
