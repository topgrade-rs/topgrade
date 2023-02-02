use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fmt::Display, rc::Rc, str::FromStr};

use color_eyre::eyre::Result;
use regex::Regex;
use strum::EnumString;
use tracing::{debug, error};

use crate::command::CommandExt;
use crate::execution_context::ExecutionContext;
use crate::terminal::print_separator;
use crate::{error::SkipStep, utils, Step};

#[derive(Debug, Copy, Clone, EnumString)]
#[strum(serialize_all = "lowercase")]
enum BoxStatus {
    PowerOff,
    Running,
    Saved,
    Aborted,
}

impl BoxStatus {
    fn powered_on(self) -> bool {
        matches!(self, BoxStatus::Running)
    }
}

#[derive(Debug)]
pub struct VagrantBox {
    path: Rc<Path>,
    name: String,
    initial_status: BoxStatus,
}

impl VagrantBox {
    pub fn smart_name(&self) -> &str {
        if self.name == "default" {
            self.path.file_name().unwrap().to_str().unwrap()
        } else {
            &self.name
        }
    }
}

impl Display for VagrantBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} @ {}", self.name, self.path.display())
    }
}

struct Vagrant {
    path: PathBuf,
}

impl Vagrant {
    fn get_boxes(&self, directory: &str) -> Result<Vec<VagrantBox>> {
        let path: Rc<Path> = Path::new(directory).into();

        let output = Command::new(&self.path)
            .arg("status")
            .current_dir(directory)
            .output_checked_utf8()?;
        debug!("Vagrant output in {}: {}", directory, output);

        let boxes = output
            .stdout
            .split('\n')
            .skip(2)
            .take_while(|line| !(line.is_empty() || line.starts_with('\r')))
            .map(|line| {
                debug!("Vagrant line: {:?}", line);
                let mut elements = line.split_whitespace();

                let name = elements.next().unwrap().to_string();
                let initial_status = BoxStatus::from_str(elements.next().unwrap()).unwrap();

                let vagrant_box = VagrantBox {
                    name,
                    path: path.clone(),
                    initial_status,
                };
                debug!("{:?}", vagrant_box);
                vagrant_box
            })
            .collect();

        Ok(boxes)
    }

    fn temporary_power_on<'a>(
        &'a self,
        vagrant_box: &'a VagrantBox,
        ctx: &'a ExecutionContext,
    ) -> Result<TemporaryPowerOn<'a>> {
        TemporaryPowerOn::create(&self.path, vagrant_box, ctx)
    }
}

struct TemporaryPowerOn<'a> {
    vagrant: &'a Path,
    vagrant_box: &'a VagrantBox,
    ctx: &'a ExecutionContext<'a>,
}

impl<'a> TemporaryPowerOn<'a> {
    fn create(vagrant: &'a Path, vagrant_box: &'a VagrantBox, ctx: &'a ExecutionContext<'a>) -> Result<Self> {
        let subcommand = match vagrant_box.initial_status {
            BoxStatus::PowerOff | BoxStatus::Aborted => "up",
            BoxStatus::Saved => "resume",
            BoxStatus::Running => unreachable!(),
        };

        ctx.run_type()
            .execute(vagrant)
            .args([subcommand, &vagrant_box.name])
            .current_dir(vagrant_box.path.clone())
            .status_checked()?;
        Ok(TemporaryPowerOn {
            vagrant,
            vagrant_box,
            ctx,
        })
    }
}

impl<'a> Drop for TemporaryPowerOn<'a> {
    fn drop(&mut self) {
        let subcommand = if self.ctx.config().vagrant_always_suspend().unwrap_or(false) {
            "suspend"
        } else {
            match self.vagrant_box.initial_status {
                BoxStatus::PowerOff | BoxStatus::Aborted => "halt",
                BoxStatus::Saved => "suspend",
                BoxStatus::Running => unreachable!(),
            }
        };

        println!();
        self.ctx
            .run_type()
            .execute(self.vagrant)
            .args([subcommand, &self.vagrant_box.name])
            .current_dir(self.vagrant_box.path.clone())
            .status_checked()
            .ok();
    }
}

pub fn collect_boxes(ctx: &ExecutionContext) -> Result<Vec<VagrantBox>> {
    let directories = utils::require_option(
        ctx.config().vagrant_directories(),
        String::from("No Vagrant directories were specified in the configuration file"),
    )?;
    let vagrant = Vagrant {
        path: utils::require("vagrant")?,
    };

    print_separator("Vagrant");
    println!("Collecting Vagrant boxes");

    let mut result = Vec::new();

    for directory in directories {
        match vagrant.get_boxes(directory) {
            Ok(mut boxes) => {
                result.append(&mut boxes);
            }
            Err(e) => error!("Error collecting vagrant boxes from {}: {}", directory, e),
        };
    }

    Ok(result)
}

pub fn topgrade_vagrant_box(ctx: &ExecutionContext, vagrant_box: &VagrantBox) -> Result<()> {
    let vagrant = Vagrant {
        path: utils::require("vagrant")?,
    };

    let seperator = format!("Vagrant ({})", vagrant_box.smart_name());
    let mut _poweron = None;
    if !vagrant_box.initial_status.powered_on() {
        if !(ctx.config().vagrant_power_on().unwrap_or(true)) {
            return Err(SkipStep(format!("Skipping powered off box {vagrant_box}")).into());
        } else {
            print_separator(seperator);
            _poweron = Some(vagrant.temporary_power_on(vagrant_box, ctx)?);
        }
    } else {
        print_separator(seperator);
    }
    let mut command = format!("env TOPGRADE_PREFIX={} topgrade", vagrant_box.smart_name());
    if ctx.config().yes(Step::Vagrant) {
        command.push_str(" -y");
    }

    ctx.run_type()
        .execute(&vagrant.path)
        .current_dir(&vagrant_box.path)
        .args(["ssh", "-c", &command])
        .status_checked()
}

pub fn upgrade_vagrant_boxes(ctx: &ExecutionContext) -> Result<()> {
    let vagrant = utils::require("vagrant")?;
    print_separator("Vagrant boxes");

    let outdated = Command::new(&vagrant)
        .args(["box", "outdated", "--global"])
        .output_checked_utf8()?;

    let re = Regex::new(r"\* '(.*?)' for '(.*?)' is outdated").unwrap();

    let mut found = false;
    for ele in re.captures_iter(&outdated.stdout) {
        found = true;
        let _ = ctx
            .run_type()
            .execute(&vagrant)
            .args(["box", "update", "--box"])
            .arg(ele.get(1).unwrap().as_str())
            .arg("--provider")
            .arg(ele.get(2).unwrap().as_str())
            .status_checked();
    }

    if !found {
        println!("No outdated boxes")
    } else {
        ctx.run_type()
            .execute(&vagrant)
            .args(["box", "prune"])
            .status_checked()?;
    }

    Ok(())
}
