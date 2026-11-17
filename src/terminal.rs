use std::cmp::{max, min};
use std::env;
use std::fmt::{self, Display};
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use anstream::adapter::strip_str;
use anstyle::{AnsiColor, Effects, Style};
use chrono::{Local, Timelike};
use color_eyre::Result;
use color_eyre::eyre;
use color_eyre::eyre::Context;
use color_eyre::eyre::OptionExt;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind, read};
use crossterm::terminal::{SetTitle, disable_raw_mode, enable_raw_mode};
#[cfg(windows)]
use crossterm_winapi::{Handle, HandleType, ScreenBuffer};
use notify_rust::{Notification, Timeout};
use rust_i18n::t;
use tracing::{debug, error};
use unicode_width::UnicodeWidthStr;

use crate::OLD_CWD;
use crate::command::CommandExt;
use crate::runner::StepResult;
#[cfg(unix)]
use crate::utils::which;
#[cfg(windows)]
use crate::utils::which_one;

static TERMINAL: LazyLock<Mutex<Terminal>> = LazyLock::new(|| Mutex::new(Terminal::new()));

static COLORS_ENABLED: LazyLock<bool> = LazyLock::new(|| {
    let stdout_is_terminal = io::stdout().is_terminal();
    // A Windows console only renders ANSI sequences once virtual terminal processing is on,
    // which `supports_ansi` turns on as it checks.
    #[cfg(windows)]
    let stdout_is_terminal = stdout_is_terminal && crossterm::ansi_support::supports_ansi();
    colors_enabled(stdout_is_terminal, |key| env::var(key).ok())
});

/// Follows the clicolors spec (<https://bixense.com/clicolors/>) plus `NO_COLOR` (<https://no-color.org/>),
/// except that an empty `NO_COLOR` also disables colors, and so does a missing or `dumb` `TERM` on unix.
fn colors_enabled(stdout_is_terminal: bool, var: impl Fn(&str) -> Option<String>) -> bool {
    #[cfg(not(windows))]
    let term_is_dumb = var("TERM").is_none_or(|term| term == "dumb");
    #[cfg(windows)]
    let term_is_dumb = false;

    var("CLICOLOR_FORCE").is_some_and(|force| force != "0")
        || (stdout_is_terminal
            && var("NO_COLOR").is_none()
            && !term_is_dumb
            && var("CLICOLOR").is_none_or(|clicolor| clicolor != "0"))
}

fn paint(content: impl Display, style: Style, enabled: bool) -> String {
    if !enabled {
        return content.to_string();
    }
    // Colors go before effects to keep topgrade's existing byte order; `Style`'s own
    // `Display` writes effects first.
    format!(
        "{}{}{content}{}",
        style.effects(Effects::new()),
        style.get_effects().render(),
        style.render_reset()
    )
}

/// Renders `content` in `style`, or plain when stdout gets no colors.
pub fn style(content: impl Display, style: Style) -> String {
    paint(content, style, *COLORS_ENABLED)
}

#[cfg(unix)]
pub fn shell() -> Result<PathBuf> {
    if let Ok(shell) = env::var("SHELL") {
        which(shell)?.ok_or_eyre("Expected binary from `SHELL` environment variable to be available")
    } else {
        which("sh")?.ok_or_eyre("Expected `sh` to be available")
    }
}

#[cfg(windows)]
pub fn shell() -> Result<PathBuf> {
    which_one(["pwsh", "powershell"])?.ok_or_eyre("Expected `pwsh` or `powershell` to be available")
}

/// `(columns, rows)` of the terminal stdout itself writes to, which can differ from the
/// controlling terminal: stdout redirected to another tty, or on Windows a pipe in front of a
/// hidden console.
#[cfg(unix)]
fn stdout_size() -> Option<(u16, u16)> {
    let size = rustix::termios::tcgetwinsize(io::stdout()).ok()?;
    Some((size.ws_col, size.ws_row))
}

#[cfg(windows)]
fn stdout_size() -> Option<(u16, u16)> {
    let size = ScreenBuffer::from(Handle::new(HandleType::OutputHandle).ok()?)
        .info()
        .ok()?
        .terminal_size();
    // `terminal_size` subtracts the window's inclusive edge coordinates.
    Some((
        u16::try_from(i32::from(size.width) + 1).ok()?,
        u16::try_from(i32::from(size.height) + 1).ok()?,
    ))
}

#[expect(clippy::disallowed_methods)]
pub fn run_shell() -> Result<()> {
    Command::new(shell()?)
        .env("IN_TOPGRADE", "1")
        .current_dir(OLD_CWD.get().expect("OLD_CWD should be set at this point"))
        .status_checked()
}

struct Terminal {
    width: Option<u16>,
    prefix: String,
    stdout_is_terminal: bool,
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
        let stdout_is_terminal = io::stdout().is_terminal();
        Self {
            // A pty whose size was never set reports 0x0.
            width: stdout_is_terminal
                .then(stdout_size)
                .flatten()
                .filter(|&(columns, rows)| columns > 0 && rows > 0)
                .map(|(columns, _)| columns),
            stdout_is_terminal,
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

    fn write(&self, args: fmt::Arguments<'_>) {
        let mut stdout = io::stdout().lock();
        stdout.write_fmt(args).and_then(|()| stdout.flush()).ok();
    }

    fn show_title(&self, title: impl Display) {
        // The title escape would otherwise land in a redirected log.
        if self.set_title && self.stdout_is_terminal {
            crossterm::execute!(io::stdout(), SetTitle(title)).ok();
        }
    }

    fn print_separator<P: AsRef<str>>(&mut self, message: P) {
        self.show_title(format_args!("{}Topgrade - {}", self.prefix, message.as_ref()));

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
                                .and_then(|e| e.checked_sub(strip_str(&message).map(UnicodeWidthStr::width).sum()))
                                .unwrap_or(0)
                        )
                    ),
                    Style::new().bold(),
                );
                self.write(format_args!("{separator}\n"));
            }
            None => {
                self.write(format_args!("―― {message} ――\n"));
            }
        }
    }

    #[allow(dead_code)]
    fn print_error<P: AsRef<str>, Q: AsRef<str>>(&mut self, key: Q, message: P) {
        let key = key.as_ref();
        let message = message.as_ref();
        self.write(format_args!(
            "{} {}",
            style(t!("{key} failed:", key = key), AnsiColor::Red.on_default().bold()),
            message
        ));
    }

    #[allow(dead_code)]
    fn print_warning<P: AsRef<str>>(&mut self, message: P) {
        let message = message.as_ref();
        self.write(format_args!(
            "{}\n",
            style(message, AnsiColor::Yellow.on_default().bold())
        ));
    }

    #[allow(dead_code)]
    fn print_info<P: AsRef<str>>(&mut self, message: P) {
        let message = message.as_ref();
        self.write(format_args!(
            "{}\n",
            style(message, AnsiColor::Blue.on_default().bold())
        ));
    }

    fn print_result<P: AsRef<str>>(&mut self, key: P, result: &StepResult) {
        let key = key.as_ref();

        self.write(format_args!(
            "{}: {}\n",
            key,
            match result {
                StepResult::Success => style(t!("OK"), AnsiColor::Green.on_default().bold()),
                StepResult::Failure => style(t!("FAILED"), AnsiColor::Red.on_default().bold()),
                StepResult::Ignored => style(t!("IGNORED"), AnsiColor::Yellow.on_default().bold()),
                StepResult::SkippedMissingSudo => format!(
                    "{}: {}",
                    style(t!("SKIPPED"), AnsiColor::Yellow.on_default().bold()),
                    t!("Could not find sudo")
                ),
                StepResult::Skipped(reason) => format!(
                    "{}: {}",
                    style(t!("SKIPPED"), AnsiColor::Blue.on_default().bold()),
                    reason
                ),
            }
        ));
    }

    #[allow(dead_code)]
    fn prompt_yesno(&mut self, question: &str) -> Result<bool, io::Error> {
        self.write(format_args!(
            "{}",
            style(
                format!("{question} {}", t!("(Y)es/(N)o")),
                AnsiColor::Yellow.on_default().bold()
            )
        ));

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

        self.show_title(format_args!("Topgrade - {}", t!("Awaiting user")));

        if self.desktop_notification {
            self.notify_desktop(format!("{}", t!("{step_name} failed", step_name = step_name)), None);
        }

        let prompt_inner = style(
            format_args!("{}{}", self.prefix, t!("Retry? (y)es/(N)o/(s)hell/(q)uit")),
            AnsiColor::Yellow.on_default().bold(),
        );

        let answer = loop {
            self.write(format_args!("\n{prompt_inner}"));
            match self.get_char() {
                Ok(KeyCode::Char('y' | 'Y')) => break Ok(ShouldRetry::Yes),
                Ok(KeyCode::Char('s' | 'S')) => {
                    println!(
                        "\n\n{}\n",
                        t!("Dropping you to shell. Fix what you need and then exit the shell.")
                    );
                    if let Err(err) = run_shell().context("Failed to run shell") {
                        self.write(format_args!("{err:?}\n{prompt_inner}"));
                    } else {
                        break Ok(ShouldRetry::Yes);
                    }
                }
                Ok(KeyCode::Char('n' | 'N') | KeyCode::Enter) => break Ok(ShouldRetry::No),
                Err(e) => {
                    if let io::ErrorKind::Interrupted = e.kind() {
                        println!();
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

        self.write(format_args!("\n"));

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

#[cfg(test)]
mod tests {
    use anstyle::{AnsiColor, Style};

    use super::{colors_enabled, paint};

    fn colors_with(stdout_is_terminal: bool, vars: &[(&str, &str)]) -> bool {
        colors_enabled(stdout_is_terminal, |key| {
            vars.iter().find(|(k, _)| *k == key).map(|(_, v)| (*v).to_string())
        })
    }

    #[test]
    fn colors_on_for_a_terminal() {
        assert!(colors_with(true, &[("TERM", "xterm-256color")]));
    }

    #[test]
    fn colors_off_when_piped() {
        assert!(!colors_with(false, &[("TERM", "xterm-256color")]));
    }

    #[test]
    fn no_color_disables_colors_even_when_empty() {
        assert!(!colors_with(true, &[("TERM", "xterm-256color"), ("NO_COLOR", "1")]));
        assert!(!colors_with(true, &[("TERM", "xterm-256color"), ("NO_COLOR", "")]));
    }

    #[test]
    fn clicolor_zero_disables_colors() {
        assert!(!colors_with(true, &[("TERM", "xterm-256color"), ("CLICOLOR", "0")]));
        assert!(colors_with(true, &[("TERM", "xterm-256color"), ("CLICOLOR", "1")]));
    }

    #[cfg(unix)]
    #[test]
    fn dumb_or_missing_term_disables_colors() {
        assert!(!colors_with(true, &[("TERM", "dumb")]));
        assert!(!colors_with(true, &[]));
    }

    #[test]
    fn clicolor_force_overrides_everything_but_zero() {
        assert!(colors_with(false, &[("CLICOLOR_FORCE", "1")]));
        assert!(colors_with(
            true,
            &[("TERM", "xterm-256color"), ("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")]
        ));
        assert!(!colors_with(false, &[("CLICOLOR_FORCE", "0")]));
    }

    #[test]
    fn empty_or_zero_force_values_match_console() {
        assert!(colors_with(false, &[("CLICOLOR_FORCE", "")]));
        assert!(colors_with(true, &[("TERM", "xterm-256color"), ("CLICOLOR", "")]));
        assert!(colors_with(
            true,
            &[("TERM", "xterm-256color"), ("CLICOLOR_FORCE", "0")]
        ));
    }

    #[cfg(windows)]
    #[test]
    fn term_is_ignored_on_windows() {
        assert!(colors_with(true, &[]));
        assert!(colors_with(true, &[("TERM", "dumb")]));
    }

    fn bold_fg(color: AnsiColor) -> Style {
        Style::new().fg_color(Some(color.into())).bold()
    }

    // expected bytes are console 0.16.6's own output for the same styles
    #[test]
    fn paint_matches_console_bytes() {
        assert_eq!(paint("x", Style::new().bold(), true), "\u{1b}[1mx\u{1b}[0m");
        assert_eq!(
            paint("OK", bold_fg(AnsiColor::Green), true),
            "\u{1b}[32m\u{1b}[1mOK\u{1b}[0m"
        );
        assert_eq!(
            paint("x", bold_fg(AnsiColor::Red), true),
            "\u{1b}[31m\u{1b}[1mx\u{1b}[0m"
        );
        assert_eq!(
            paint("x", bold_fg(AnsiColor::Yellow), true),
            "\u{1b}[33m\u{1b}[1mx\u{1b}[0m"
        );
        assert_eq!(
            paint("x", bold_fg(AnsiColor::Blue), true),
            "\u{1b}[34m\u{1b}[1mx\u{1b}[0m"
        );
        assert_eq!(
            paint("x", bold_fg(AnsiColor::Cyan), true),
            "\u{1b}[36m\u{1b}[1mx\u{1b}[0m"
        );
    }

    #[test]
    fn paint_is_plain_when_colors_are_off() {
        assert_eq!(paint("OK", bold_fg(AnsiColor::Green), false), "OK");
    }
}
