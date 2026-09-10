use std::cmp::{max, min};
use std::env;
use std::fmt::Display;
use std::io::{self, IsTerminal, Write};
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use chrono::{Local, Timelike};
use color_eyre::eyre;
use color_eyre::eyre::Context;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind, read};
use crossterm::style::{StyledContent, Stylize};
use crossterm::terminal::{SetTitle, disable_raw_mode, enable_raw_mode, size};
use notify_rust::{Notification, Timeout};
use rust_i18n::t;
use tracing::{debug, error};
use unicode_width::UnicodeWidthStr;
#[cfg(windows)]
use which_crate::which;

use crate::command::CommandExt;
use crate::runner::StepResult;

static TERMINAL: LazyLock<Mutex<Terminal>> = LazyLock::new(|| Mutex::new(Terminal::new()));

/// Whether styled output should emit ANSI sequences.
///
/// Restores the suppression `console` did automatically: honor `CLICOLOR_FORCE`, disable on
/// `NO_COLOR`, otherwise colorize only when stdout is a terminal. crossterm never strips
/// sequences on its own, so the decision is made here and applied via [`style`].
static COLORED: LazyLock<bool> = LazyLock::new(|| {
    if env::var("CLICOLOR_FORCE").is_ok_and(|v| !v.is_empty() && v != "0") {
        return true;
    }
    if env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return false;
    }
    io::stdout().is_terminal()
});

/// Renders `content` with `style_fn` applied, or plain when color output is disabled.
pub fn style<D: Display>(content: D, style_fn: impl FnOnce(StyledContent<D>) -> StyledContent<D>) -> String {
    if *COLORED {
        style_fn(crossterm::style::style(content)).to_string()
    } else {
        content.to_string()
    }
}

#[cfg(unix)]
pub fn shell() -> String {
    env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
}

#[cfg(windows)]
pub fn shell() -> &'static str {
    which("pwsh").map(|_| "pwsh").unwrap_or("powershell")
}

#[expect(clippy::disallowed_methods)]
pub fn run_shell() -> eyre::Result<()> {
    Command::new(shell()).env("IN_TOPGRADE", "1").status_checked()
}

struct Terminal {
    width: Option<u16>,
    prefix: String,
    term: io::Stdout,
    set_title: bool,
    display_time: bool,
    desktop_notification: bool,
}

struct RawTerminalMode;

impl RawTerminalMode {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let guard = Self;
        crossterm::execute!(io::stdout(), EnableBracketedPaste)?;
        Ok(guard)
    }
}

impl Drop for RawTerminalMode {
    fn drop(&mut self) {
        crossterm::execute!(io::stdout(), DisableBracketedPaste).unwrap();
        disable_raw_mode().unwrap();
    }
}

impl Terminal {
    fn new() -> Self {
        Self {
            width: size().map(|(w, _)| w).ok(),
            term: io::stdout(),
            prefix: env::var("TOPGRADE_PREFIX").map_or_else(|_| String::new(), |prefix| format!("({prefix}) ")),
            set_title: true,
            display_time: true,
            desktop_notification: false,
        }
    }

    fn set_desktop_notifications(&mut self, desktop_notifications: bool) {
        self.desktop_notification = desktop_notifications;
    }

    fn set_title(&mut self, set_title: bool) {
        self.set_title = set_title;
    }

    fn display_time(&mut self, display_time: bool) {
        self.display_time = display_time;
    }

    fn notify_desktop<P: AsRef<str>>(&self, message: P, timeout: Option<Duration>) {
        debug!("Desktop notification: {}", message.as_ref());
        let mut notification = Notification::new();
        notification
            .summary("Topgrade")
            .body(message.as_ref())
            .appname("topgrade");

        if let Some(timeout) = timeout {
            notification.timeout(Timeout::Milliseconds(timeout.as_millis() as u32));
        }
        notification.show().ok();
    }

    fn print_separator<P: AsRef<str>>(&mut self, message: P) {
        if self.set_title {
            crossterm::execute!(
                self.term,
                SetTitle(format!("{}Topgrade - {}", self.prefix, message.as_ref()))
            )
            .ok();
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
                let separator = style(
                    format_args!(
                        "\n── {} {:─^border$}",
                        message,
                        "",
                        border = max(
                            2,
                            min(80, width as usize)
                                .checked_sub(4)
                                .and_then(|e| e.checked_sub(message.width()))
                                .unwrap_or(0)
                        )
                    ),
                    |s| s.bold(),
                );
                self.term.write_fmt(format_args!("{separator}\n")).ok();
            }
            None => {
                self.term.write_fmt(format_args!("―― {message} ――\n")).ok();
            }
        }
    }

    #[allow(dead_code)]
    fn print_error<P: AsRef<str>, Q: AsRef<str>>(&mut self, key: Q, message: P) {
        let key = key.as_ref();
        let message = message.as_ref();
        self.term
            .write_fmt(format_args!(
                "{} {}",
                style(t!("{key} failed:", key = key), |s| s.red().bold()),
                message
            ))
            .ok();
    }

    #[allow(dead_code)]
    fn print_warning<P: AsRef<str>>(&mut self, message: P) {
        let message = message.as_ref();
        self.term
            .write_fmt(format_args!("{}\n", style(message, |s| s.yellow().bold())))
            .ok();
    }

    #[allow(dead_code)]
    fn print_info<P: AsRef<str>>(&mut self, message: P) {
        let message = message.as_ref();
        self.term
            .write_fmt(format_args!("{}\n", style(message, |s| s.blue().bold())))
            .ok();
    }

    fn print_result<P: AsRef<str>>(&mut self, key: P, result: &StepResult) {
        let key = key.as_ref();

        self.term
            .write_fmt(format_args!(
                "{}: {}\n",
                key,
                match result {
                    StepResult::Success => style(t!("OK"), |s| s.bold().green()),
                    StepResult::Failure => style(t!("FAILED"), |s| s.bold().red()),
                    StepResult::Ignored => style(t!("IGNORED"), |s| s.bold().yellow()),
                    StepResult::SkippedMissingSudo => format!(
                        "{}: {}",
                        style(t!("SKIPPED"), |s| s.bold().yellow()),
                        t!("Could not find sudo")
                    ),
                    StepResult::Skipped(reason) => format!("{}: {}", style(t!("SKIPPED"), |s| s.bold().blue()), reason),
                }
            ))
            .ok();
    }

    #[allow(dead_code)]
    fn prompt_yesno(&mut self, question: &str) -> Result<bool, io::Error> {
        self.term
            .write_fmt(format_args!(
                "{}",
                style(format!("{question} {}", t!("(Y)es/(N)o")), |s| s.yellow().bold())
            ))
            .ok();

        loop {
            match self.get_char()? {
                KeyCode::Char('y' | 'Y') => break Ok(true),
                KeyCode::Char('n' | 'N') | KeyCode::Enter => break Ok(false),
                _ => (),
            }
        }
    }

    fn should_retry(&mut self, step_name: &str) -> eyre::Result<ShouldRetry> {
        if self.width.is_none() {
            return Ok(ShouldRetry::No);
        }

        if self.set_title {
            crossterm::execute!(self.term, SetTitle(format!("Topgrade - {}", t!("Awaiting user")))).ok();
        }

        if self.desktop_notification {
            self.notify_desktop(format!("{}", t!("{step_name} failed", step_name = step_name)), None);
        }

        let prompt_inner = style(
            format!("{}{}", self.prefix, t!("Retry? (y)es/(N)o/(s)hell/(q)uit")),
            |s| s.yellow().bold(),
        );

        let answer = loop {
            self.term.write_fmt(format_args!("\n{prompt_inner}")).ok();
            match self.get_char() {
                Ok(KeyCode::Char('y' | 'Y')) => break Ok(ShouldRetry::Yes),
                Ok(KeyCode::Char('s' | 'S')) => {
                    self.term
                        .write_fmt(format_args!(
                            "\n\n{}\n",
                            t!("Dropping you to shell. Fix what you need and then exit the shell.")
                        ))
                        .ok();
                    if let Err(err) = run_shell().context("Failed to run shell") {
                        self.term.write_fmt(format_args!("{err:?}\n{prompt_inner}")).ok();
                    } else {
                        break Ok(ShouldRetry::Yes);
                    }
                }
                Ok(KeyCode::Char('n' | 'N') | KeyCode::Enter) => break Ok(ShouldRetry::No),
                Err(e) => {
                    if let io::ErrorKind::Interrupted = e.kind() {
                        self.term.write_all(b"\n").ok();
                        error!("Interrupted while reading from terminal: {}", e);
                        continue;
                    }
                    error!("Error reading from terminal: {}", e);
                    break Ok(ShouldRetry::No);
                }
                Ok(KeyCode::Char('q' | 'Q')) => {
                    break Ok(ShouldRetry::Quit);
                }
                _ => (),
            }
        };

        self.term.write_all(b"\n").ok();

        answer
    }

    fn get_char(&self) -> io::Result<KeyCode> {
        let _raw_mode_guard = RawTerminalMode::enter()?;
        loop {
            let Event::Key(key) = read()? else { continue };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            break Ok(key.code);
        }
    }
}

#[derive(Clone, Copy)]
pub enum ShouldRetry {
    Yes,
    No,
    Quit,
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

pub fn should_retry(step_name: &str) -> eyre::Result<ShouldRetry> {
    TERMINAL.lock().unwrap().should_retry(step_name)
}

pub fn print_separator<P: AsRef<str>>(message: P) {
    TERMINAL.lock().unwrap().print_separator(message);
}

#[allow(dead_code)]
pub fn print_error<P: AsRef<str>, Q: AsRef<str>>(key: Q, message: P) {
    TERMINAL.lock().unwrap().print_error(key, message);
}

#[allow(dead_code)]
pub fn print_warning<P: AsRef<str>>(message: P) {
    TERMINAL.lock().unwrap().print_warning(message);
}

#[allow(dead_code)]
pub fn print_info<P: AsRef<str>>(message: P) {
    TERMINAL.lock().unwrap().print_info(message);
}

pub fn print_result<P: AsRef<str>>(key: P, result: &StepResult) {
    TERMINAL.lock().unwrap().print_result(key, result);
}

/// Tells whether the terminal is dumb.
pub fn is_dumb() -> bool {
    TERMINAL.lock().unwrap().width.is_none()
}

pub fn get_key() -> io::Result<KeyCode> {
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
    TERMINAL.lock().unwrap().notify_desktop(message, timeout);
}

pub fn display_time(display_time: bool) {
    TERMINAL.lock().unwrap().display_time(display_time);
}
