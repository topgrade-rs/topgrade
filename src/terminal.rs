use std::cmp::{max, min};
use std::env;
use std::io::{self, Write};
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use chrono::{Local, Timelike};
use console::{style, Key, Term};
use lazy_static::lazy_static;
use log::{debug, error};
#[cfg(target_os = "macos")]
use notify_rust::{Notification, Timeout};
#[cfg(windows)]
use which_crate::which;

use crate::report::StepResult;
#[cfg(target_os = "linux")]
use crate::utils::which;

lazy_static! {
    static ref TERMINAL: Mutex<Terminal> = Mutex::new(Terminal::new());
}

#[cfg(unix)]
pub fn shell() -> String {
    env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
}

#[cfg(windows)]
pub fn shell() -> &'static str {
    which("pwsh").map(|_| "pwsh").unwrap_or("powershell")
}

pub fn run_shell() {
    Command::new(shell())
        .env("IN_TOPGRADE", "1")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

struct Terminal {
    width: Option<u16>,
    prefix: String,
    term: Term,
    set_title: bool,
    display_time: bool,
    desktop_notification: bool,
    #[cfg(target_os = "linux")]
    notify_send: Option<PathBuf>,
}

impl Terminal {
    fn new() -> Self {
        let term = Term::stdout();
        Self {
            width: term.size_checked().map(|(_, w)| w),
            term,
            prefix: env::var("TOPGRADE_PREFIX")
                .map(|prefix| format!("({}) ", prefix))
                .unwrap_or_else(|_| String::new()),
            set_title: true,
            display_time: true,
            desktop_notification: false,
            #[cfg(target_os = "linux")]
            notify_send: which("notify-send"),
        }
    }

    fn set_desktop_notifications(&mut self, desktop_notifications: bool) {
        self.desktop_notification = desktop_notifications
    }

    fn set_title(&mut self, set_title: bool) {
        self.set_title = set_title
    }

    fn display_time(&mut self, display_time: bool) {
        self.display_time = display_time
    }

    #[allow(unused_variables)]
    fn notify_desktop<P: AsRef<str>>(&self, message: P, timeout: Option<Duration>) {
        debug!("Desktop notification: {}", message.as_ref());
        cfg_if::cfg_if! {
            if #[cfg(target_os = "macos")] {
                let mut notification = Notification::new();
                notification.summary("Topgrade")
                    .body(message.as_ref())
                    .appname("topgrade");

                if let Some(timeout) = timeout {
                    notification.timeout(Timeout::Milliseconds(timeout.as_millis() as u32));
                }
                notification.show().ok();
            } else if #[cfg(target_os = "linux")] {
                if let Some(ns) = self.notify_send.as_ref() {
                    let mut command = Command::new(ns);
                    if let Some(timeout) = timeout {
                        command.arg("-t");
                        command.arg(format!("{}", timeout.as_millis()));
                    }
                    command.args(["-a", "Topgrade", "Topgrade"]);
                    command.arg(message.as_ref());
                    command.output().ok();
                }
            }
        }
    }

    fn print_separator<P: AsRef<str>>(&mut self, message: P) {
        if self.set_title {
            self.term
                .set_title(format!("{}Topgrade - {}", self.prefix, message.as_ref()));
        }

        if self.desktop_notification {
            self.notify_desktop(message.as_ref(), Some(Duration::from_secs(5)));
        }

        let now = Local::now();
        let message = if self.display_time {
            format!(
                "{}{:02}:{:02}:{:02} - {}",
                self.prefix,
                now.hour(),
                now.minute(),
                now.second(),
                message.as_ref()
            )
        } else {
            String::from(message.as_ref())
        };

        match self.width {
            Some(width) => {
                self.term
                    .write_fmt(format_args!(
                        "{}\n",
                        style(format_args!(
                            "\n―― {} {:―^border$}",
                            message,
                            "",
                            border = max(
                                2,
                                min(80, width as usize)
                                    .checked_sub(4)
                                    .and_then(|e| e.checked_sub(message.len()))
                                    .unwrap_or(0)
                            )
                        ))
                        .bold()
                    ))
                    .ok();
            }
            None => {
                self.term.write_fmt(format_args!("―― {} ――\n", message)).ok();
            }
        }
    }

    #[allow(dead_code)]
    fn print_warning<P: AsRef<str>>(&mut self, message: P) {
        let message = message.as_ref();
        self.term
            .write_fmt(format_args!("{}\n", style(message).yellow().bold()))
            .ok();
    }

    #[allow(dead_code)]
    fn print_info<P: AsRef<str>>(&mut self, message: P) {
        let message = message.as_ref();
        self.term
            .write_fmt(format_args!("{}\n", style(message).blue().bold()))
            .ok();
    }

    fn print_result<P: AsRef<str>>(&mut self, key: P, result: &StepResult) {
        let key = key.as_ref();

        self.term
            .write_fmt(format_args!(
                "{}: {}\n",
                key,
                match result {
                    StepResult::Success => format!("{}", style("OK").bold().green()),
                    StepResult::Failure => format!("{}", style("FAILED").bold().red()),
                    StepResult::Ignored => format!("{}", style("IGNORED").bold().yellow()),
                    StepResult::Skipped(reason) => format!("{}: {}", style("SKIPPED").bold().blue(), reason),
                }
            ))
            .ok();
    }

    #[allow(dead_code)]
    fn prompt_yesno(&mut self, question: &str) -> Result<bool, io::Error> {
        self.term
            .write_fmt(format_args!(
                "{}",
                style(format!("{} (y)es/(N)o", question,)).yellow().bold()
            ))
            .ok();

        loop {
            match self.term.read_char()? {
                'y' | 'Y' => break Ok(true),
                'n' | 'N' | '\r' | '\n' => break Ok(false),
                _ => (),
            }
        }
    }
    #[allow(unused_variables)]
    fn should_retry(&mut self, interrupted: bool, step_name: &str) -> Result<bool, io::Error> {
        if self.width.is_none() {
            return Ok(false);
        }

        if self.set_title {
            self.term.set_title("Topgrade - Awaiting user");
        }

        self.notify_desktop(format!("{} failed", step_name), None);

        self.term
            .write_fmt(format_args!(
                "\n{}",
                style(format!("{}Retry? (y)es/(N)o/(s)hell/(q)uit", self.prefix))
                    .yellow()
                    .bold()
            ))
            .ok();

        let answer = loop {
            match self.term.read_key() {
                Ok(Key::Char('y')) | Ok(Key::Char('Y')) => break Ok(true),
                Ok(Key::Char('s')) | Ok(Key::Char('S')) => {
                    println!("\n\nDropping you to shell. Fix what you need and then exit the shell.\n");
                    run_shell();
                    break Ok(true);
                }
                Ok(Key::Char('n')) | Ok(Key::Char('N')) | Ok(Key::Enter) => break Ok(false),
                Err(e) => {
                    error!("Error reading from terminal: {}", e);
                    break Ok(false);
                }
                Ok(Key::Char('q')) | Ok(Key::Char('Q')) => return Err(io::Error::from(io::ErrorKind::Interrupted)),
                _ => (),
            }
        };

        self.term.write_str("\n").ok();

        answer
    }

    fn get_char(&self) -> Result<Key, io::Error> {
        self.term.read_key()
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

pub fn should_retry(interrupted: bool, step_name: &str) -> Result<bool, io::Error> {
    TERMINAL.lock().unwrap().should_retry(interrupted, step_name)
}

pub fn print_separator<P: AsRef<str>>(message: P) {
    TERMINAL.lock().unwrap().print_separator(message)
}

#[allow(dead_code)]
pub fn print_warning<P: AsRef<str>>(message: P) {
    TERMINAL.lock().unwrap().print_warning(message)
}

#[allow(dead_code)]
pub fn print_info<P: AsRef<str>>(message: P) {
    TERMINAL.lock().unwrap().print_info(message)
}

pub fn print_result<P: AsRef<str>>(key: P, result: &StepResult) {
    TERMINAL.lock().unwrap().print_result(key, result)
}

/// Tells whether the terminal is dumb.
pub fn is_dumb() -> bool {
    TERMINAL.lock().unwrap().width.is_none()
}

pub fn get_key() -> Result<Key, io::Error> {
    TERMINAL.lock().unwrap().get_char()
}

pub fn set_title(set_title: bool) {
    TERMINAL.lock().unwrap().set_title(set_title);
}

pub fn set_desktop_notifications(desktop_notifications: bool) {
    TERMINAL
        .lock()
        .unwrap()
        .set_desktop_notifications(desktop_notifications);
}

#[allow(dead_code)]
pub fn prompt_yesno(question: &str) -> Result<bool, io::Error> {
    TERMINAL.lock().unwrap().prompt_yesno(question)
}

pub fn notify_desktop<P: AsRef<str>>(message: P, timeout: Option<Duration>) {
    TERMINAL.lock().unwrap().notify_desktop(message, timeout)
}

pub fn display_time(display_time: bool) {
    TERMINAL.lock().unwrap().display_time(display_time);
}
