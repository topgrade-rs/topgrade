use crate::executor::RunType;
use crate::terminal::print_separator;
use crate::{
    execution_context::ExecutionContext,
    utils::{which, Check, PathExt},
};
use anyhow::Result;
use directories::BaseDirs;
use std::env;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{exit, Command};

pub fn run_tpm(base_dirs: &BaseDirs, run_type: RunType) -> Result<()> {
    let tpm = base_dirs
        .home_dir()
        .join(".tmux/plugins/tpm/bin/update_plugins")
        .require()?;

    print_separator("tmux plugins");

    run_type.execute(tpm).arg("all").check_run()
}

struct Tmux {
    tmux: PathBuf,
    args: Option<Vec<String>>,
}

impl Tmux {
    fn new(args: &Option<String>) -> Self {
        Self {
            tmux: which("tmux").expect("Could not find tmux"),
            args: args
                .as_ref()
                .map(|args| args.split_whitespace().map(String::from).collect()),
        }
    }

    fn build(&self) -> Command {
        let mut command = Command::new(&self.tmux);
        if let Some(args) = self.args.as_ref() {
            command.args(args).env_remove("TMUX");
        }
        command
    }

    fn has_session(&self, session_name: &str) -> Result<bool, io::Error> {
        Ok(self
            .build()
            .args(["has-session", "-t", session_name])
            .output()?
            .status
            .success())
    }

    fn new_session(&self, session_name: &str) -> Result<bool, io::Error> {
        Ok(self
            .build()
            .args(["new-session", "-d", "-s", session_name, "-n", "dummy"])
            .spawn()?
            .wait()?
            .success())
    }

    fn run_in_session(&self, command: &str) -> Result<()> {
        self.build()
            .args(["new-window", "-t", "topgrade", command])
            .spawn()?
            .wait()?
            .check()?;

        Ok(())
    }
}

pub fn run_in_tmux(args: &Option<String>) -> ! {
    let command = {
        let mut command = vec![
            String::from("env"),
            String::from("TOPGRADE_KEEP_END=1"),
            String::from("TOPGRADE_INSIDE_TMUX=1"),
        ];
        command.extend(env::args());
        command.join(" ")
    };

    let tmux = Tmux::new(args);

    if !tmux.has_session("topgrade").expect("Error detecting a tmux session") {
        tmux.new_session("topgrade").expect("Error creating a tmux session");
    }

    tmux.run_in_session(&command).expect("Error running Topgrade in tmux");
    tmux.build()
        .args(["kill-window", "-t", "topgrade:dummy"])
        .output()
        .expect("Error killing the dummy tmux window");

    if env::var("TMUX").is_err() {
        let err = tmux.build().args(["attach", "-t", "topgrade"]).exec();
        panic!("{:?}", err);
    } else {
        println!("Topgrade launched in a new tmux session");
        exit(0);
    }
}

pub fn run_command(ctx: &ExecutionContext, command: &str) -> Result<()> {
    Tmux::new(ctx.config().tmux_arguments())
        .build()
        .args(["new-window", "-a", "-t", "topgrade:1", command])
        .env_remove("TMUX")
        .spawn()?
        .wait()?
        .check()
}
