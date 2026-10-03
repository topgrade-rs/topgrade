//! Run a child on its own pseudo-terminal, proxying our terminal to it.
//!
//! Homebrew's `brew.sh` runs `sudo --reset-timestamp` unconditionally at startup (#2138). The
//! reset is keyed to sudo's *controlling terminal*, so running brew on ours drops the credentials
//! `pre_sudo`/`sudo_loop` cached and re-prompts at the next sudo step. A private pty confines the
//! reset to the pty's own timestamp record; because it is a real terminal (not `setsid`'s absent
//! one), brew's own `sudo` prompt during a cask install stays answerable through the proxy.
use std::io::{IsTerminal, stderr, stdin, stdout};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicI32, Ordering};

use color_eyre::eyre::{Context, Result, eyre};
use nix::errno::Errno;
use nix::libc;
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::pty::{OpenptyResult, Winsize, openpty};
use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction};
use nix::sys::termios::{SetArg, SpecialCharacterIndices, Termios, cfmakeraw, tcgetattr, tcsetattr};
use nix::unistd::{read, write};

/// How long the proxy blocks in `poll` before re-checking whether the child is gone. Bounded rather
/// than infinite because a grandchild holding the slave open leaves the pty both quiet and un-closed.
const POLL_TIMEOUT_MS: u16 = 250;

/// Write end of the self-pipe a `SIGWINCH` wakes the proxy poll loop through, or `-1` when no proxy
/// is active. A pipe (not a bare flag) is what lets a blocking `poll` return on a resize.
static SIGWINCH_PIPE_W: AtomicI32 = AtomicI32::new(-1);

extern "C" fn handle_sigwinch(_: i32) {
    let fd = SIGWINCH_PIPE_W.load(Ordering::SeqCst);
    if fd >= 0 {
        // SAFETY: `write(2)` is async-signal-safe. The pipe is non-blocking, so a full one fails
        // with EAGAIN and drops the wakeup rather than stalling the handler.
        unsafe {
            libc::write(fd, [0u8].as_ptr().cast(), 1);
        }
    }
}

/// Run `cmd` on a private pty, forwarding our terminal both ways, and return its exit status.
///
/// Falls back to a plain foreground spawn unless all three standard streams are a terminal. With
/// stdio redirected there is no interactive terminal to proxy, so brew runs where the user pointed
/// it; a controlling terminal that survives the redirection still gets its timestamp reset. Honoring
/// the redirection wins over confining the reset in that case.
pub fn spawn_on_pty(cmd: &mut Command) -> Result<ExitStatus> {
    if !stdin().is_terminal() || !stdout().is_terminal() || !stderr().is_terminal() {
        #[allow(clippy::disallowed_methods)]
        return cmd.status().map_err(Into::into);
    }

    let our_tty = stdin();
    let original = tcgetattr(our_tty.as_fd()).context("tcgetattr")?;
    let vintr = original.control_chars[SpecialCharacterIndices::VINTR as usize];
    let winsize = get_winsize(our_tty.as_fd());
    // Hand the child a terminal configured like ours, so its line discipline (ISIG, VINTR) matches
    // what the user actually has rather than the kernel defaults.
    let OpenptyResult { master, slave } = openpty(winsize.as_ref(), Some(&original)).context("openpty failed")?;
    set_cloexec(master.as_fd());

    let slave_fd = slave.as_raw_fd();
    // Redirect in the child rather than handing the slave to `Command::stdio`: a `Command` stores
    // those fds and outlives `spawn`, so the parent would keep slave copies open and the master
    // would never report EOF when the child exits.
    //
    // SAFETY: `setsid`, `ioctl`, `dup2` and `close` are async-signal-safe syscalls valid between
    // fork and exec. `setsid` makes the child a session leader with no controlling terminal, so
    // `TIOCSCTTY` on the slave claims it as the ctty (and foreground pgrp), re-keying brew's reset.
    unsafe {
        cmd.pre_exec(move || {
            nix::unistd::setsid()?;
            if libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            for target in 0..=2 {
                if libc::dup2(slave_fd, target) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            if slave_fd > 2 {
                libc::close(slave_fd);
            }
            Ok(())
        });
    }

    // Every guard is installed before spawning: a failure afterwards would drop `Child` without
    // reaping it (std neither waits nor kills), orphaning brew on a pty nobody drains.
    let _ignore = SigIgnoreGuard::install()?;
    let _raw = RawTermGuard::enter(our_tty.as_fd(), original)?;
    let _winch = SigwinchGuard::install()?;

    #[allow(clippy::disallowed_methods)]
    let mut child = cmd.spawn().context("spawn on pty failed")?;
    // The parent's last slave copy: the master only reports EOF once every slave fd is closed.
    drop(slave);

    let result = proxy(master.as_fd(), &mut child, &_winch, vintr);
    if result.is_err() {
        // Nothing is draining the pty any more, so a still-running brew would block writing to it
        // and `wait` would never return.
        let _ = child.kill();
    }
    let status = child.wait().context("waiting on pty child")?;
    result?;

    // A child killed outright by SIGINT (rather than one that caught it, see `proxy`) still means
    // the run was interrupted.
    if status.signal() == Some(libc::SIGINT) {
        crate::ctrlc::set_interrupted();
    }
    Ok(status)
}

/// Shuttle bytes between our terminal and the pty master until the child closes its side.
///
/// `vintr` is the terminal's interrupt character, or `0` when disabled.
fn proxy(master: BorrowedFd, child: &mut Child, sig: &SigwinchGuard, vintr: u8) -> Result<()> {
    let stdin = stdin();
    let stdout = stdout();
    let mut buf = [0u8; 4096];
    let mut watch_stdin = true;

    loop {
        let mut fds = [
            PollFd::new(master, PollFlags::POLLIN),
            PollFd::new(sig.read_fd(), PollFlags::POLLIN),
            PollFd::new(
                if watch_stdin { stdin.as_fd() } else { master },
                if watch_stdin {
                    PollFlags::POLLIN
                } else {
                    PollFlags::empty()
                },
            ),
        ];
        match poll(&mut fds, PollTimeout::from(POLL_TIMEOUT_MS)) {
            Ok(_) | Err(Errno::EINTR) => {}
            Err(e) => return Err(e).context("poll pty"),
        }

        let revents = |i: usize| fds[i].revents().unwrap_or(PollFlags::empty());

        if revents(1).contains(PollFlags::POLLIN) {
            let mut drain_buf = [0u8; 64];
            let _ = read(sig.read_fd(), &mut drain_buf);
            if let Some(ws) = get_winsize(stdin.as_fd()) {
                set_winsize(master, &ws);
            }
        }

        let mut master_had_data = false;
        if revents(0).intersects(PollFlags::POLLIN | PollFlags::POLLHUP | PollFlags::POLLERR) {
            match read(master, &mut buf) {
                Ok(0) => break, // child closed the pty
                Ok(n) => {
                    write_all(stdout.as_fd(), &buf[..n])?;
                    master_had_data = true;
                }
                Err(Errno::EINTR) => {}
                Err(Errno::EIO) => break, // last slave fd is gone
                Err(e) => return Err(e).context("reading pty"),
            }
        }

        if watch_stdin && revents(2).contains(PollFlags::POLLIN) {
            match read(stdin.as_fd(), &mut buf) {
                Ok(0) => watch_stdin = false, // our stdin closed; keep relaying child output
                Ok(n) => {
                    // Raw mode means Ctrl-C reaches us as a byte instead of a signal, and the child
                    // may catch it and exit normally (brew is a shell script, so exit 130), leaving
                    // nothing in its status to detect. Flag the interrupt here so the runner prompts
                    // rather than silently auto-retrying the step.
                    // `_POSIX_VDISABLE` is 0 on Linux but 0xff on macOS/BSD; both mean "no VINTR".
                    if vintr != 0 && vintr != 0xff && buf[..n].contains(&vintr) {
                        crate::ctrlc::set_interrupted();
                    }
                    write_all(master, &buf[..n])?;
                }
                Err(Errno::EINTR) => {}
                Err(_) => watch_stdin = false,
            }
        }

        // Only once the pty has gone quiet: its buffer outlives the child, so leaving while output
        // is still queued would truncate the tail of brew's log. Reaching here at all needs the
        // bounded poll timeout, since a grandchild can hold the slave open without ever writing.
        if !master_had_data && matches!(child.try_wait(), Ok(Some(_))) {
            drain(master, stdout.as_fd())?;
            break;
        }
    }
    Ok(())
}

/// Read whatever is still queued on `master` without blocking.
fn drain(master: BorrowedFd, out: BorrowedFd) -> Result<()> {
    let mut buf = [0u8; 4096];
    loop {
        let mut fds = [PollFd::new(master, PollFlags::POLLIN)];
        if !matches!(poll(&mut fds, PollTimeout::ZERO), Ok(n) if n > 0) {
            return Ok(());
        }
        if !fds[0]
            .revents()
            .unwrap_or(PollFlags::empty())
            .intersects(PollFlags::POLLIN | PollFlags::POLLHUP)
        {
            return Ok(());
        }
        match read(master, &mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => write_all(out, &buf[..n])?,
            Err(Errno::EINTR) => {}
            Err(_) => return Ok(()),
        }
    }
}

fn write_all(fd: BorrowedFd, mut data: &[u8]) -> Result<()> {
    while !data.is_empty() {
        match write(fd, data) {
            Ok(0) => return Err(eyre!("short write to terminal")),
            Ok(n) => data = &data[n..],
            Err(Errno::EINTR) => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Keep the fd out of the child: `openpty` does not set close-on-exec, so brew (and the `sudo` it
/// runs) would otherwise inherit a writable handle to our terminal proxy.
fn set_cloexec(fd: BorrowedFd) {
    // SAFETY: `F_SETFD` takes an int flag; failure only leaves the previous (inherited) state.
    unsafe {
        libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC);
    }
}

fn get_winsize(fd: BorrowedFd) -> Option<Winsize> {
    // SAFETY: `winsize` is a plain C struct of integers, so an all-zero value is valid to hand to
    // the ioctl below, which overwrites it.
    let mut ws: Winsize = unsafe { std::mem::zeroed() };
    // SAFETY: `TIOCGWINSZ` writes a `winsize` into `ws`; a non-tty fd just returns an error.
    if unsafe { libc::ioctl(fd.as_raw_fd(), libc::TIOCGWINSZ as _, &mut ws) } == 0 {
        Some(ws)
    } else {
        None
    }
}

fn set_winsize(fd: BorrowedFd, ws: &Winsize) {
    // SAFETY: `TIOCSWINSZ` reads a `winsize`; it also sends SIGWINCH to the pty's fg group, which is
    // how the child learns of the resize.
    unsafe {
        libc::ioctl(fd.as_raw_fd(), libc::TIOCSWINSZ as _, ws);
    }
}

/// Ignores `SIGTTOU`/`SIGTTIN` while we drive the terminal, restoring them on drop.
///
/// If topgrade is not the terminal's foreground process group (e.g. launched without job control),
/// `tcsetattr` and our reads would otherwise raise those signals and stop the process. This is a
/// guard of its own so the dispositions are restored even when the termios setup that follows fails.
struct SigIgnoreGuard {
    prev_ttou: SigAction,
    prev_ttin: SigAction,
}

impl SigIgnoreGuard {
    fn install() -> Result<Self> {
        let ignore = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
        // SAFETY: installing SIG_IGN; captured dispositions are restored on drop.
        unsafe {
            let prev_ttou = sigaction(Signal::SIGTTOU, &ignore).context("ignore SIGTTOU")?;
            match sigaction(Signal::SIGTTIN, &ignore) {
                Ok(prev_ttin) => Ok(Self { prev_ttou, prev_ttin }),
                Err(e) => {
                    let _ = sigaction(Signal::SIGTTOU, &prev_ttou);
                    Err(e).context("ignore SIGTTIN")
                }
            }
        }
    }
}

impl Drop for SigIgnoreGuard {
    fn drop(&mut self) {
        // SAFETY: restoring the dispositions captured at install time.
        unsafe {
            let _ = sigaction(Signal::SIGTTOU, &self.prev_ttou);
            let _ = sigaction(Signal::SIGTTIN, &self.prev_ttin);
        }
    }
}

/// Puts our terminal in raw mode for the proxy's lifetime and restores it on drop (including on
/// panic), so a failed brew run never leaves the shell with echo off.
struct RawTermGuard {
    fd: RawFd,
    original: Termios,
}

impl RawTermGuard {
    fn enter(fd: BorrowedFd, original: Termios) -> Result<Self> {
        let mut raw = original.clone();
        cfmakeraw(&mut raw);
        tcsetattr(fd, SetArg::TCSANOW, &raw).context("tcsetattr raw")?;
        Ok(Self {
            fd: fd.as_raw_fd(),
            original,
        })
    }
}

impl Drop for RawTermGuard {
    fn drop(&mut self) {
        // SAFETY: reconstructs a borrow of the still-open terminal fd only to restore its attrs.
        let fd = unsafe { BorrowedFd::borrow_raw(self.fd) };
        let _ = tcsetattr(fd, SetArg::TCSANOW, &self.original);
    }
}

/// Installs a `SIGWINCH` handler wired to a self-pipe, and tears it back down on drop so the handler
/// never fires against a closed pipe fd.
struct SigwinchGuard {
    read: OwnedFd,
    _write: OwnedFd,
    previous: SigAction,
}

impl SigwinchGuard {
    fn install() -> Result<Self> {
        let (read, write) = nix::unistd::pipe().context("sigwinch pipe")?;
        // Non-blocking so the signal handler can never stall on a full pipe; close-on-exec so the
        // wakeup channel does not leak into brew.
        for fd in [read.as_fd(), write.as_fd()] {
            set_cloexec(fd);
            // SAFETY: `F_SETFL` takes an int flag on an fd we own.
            unsafe {
                libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
            }
        }
        SIGWINCH_PIPE_W.store(write.as_raw_fd(), Ordering::SeqCst);
        let action = SigAction::new(
            SigHandler::Handler(handle_sigwinch),
            SaFlags::SA_RESTART,
            SigSet::empty(),
        );
        // SAFETY: the handler only writes one byte to the self-pipe (async-signal-safe).
        let previous = unsafe { sigaction(Signal::SIGWINCH, &action).context("install SIGWINCH")? };
        Ok(Self {
            read,
            _write: write,
            previous,
        })
    }

    fn read_fd(&self) -> BorrowedFd<'_> {
        self.read.as_fd()
    }
}

impl Drop for SigwinchGuard {
    fn drop(&mut self) {
        // Restore the disposition before clearing the fd, so no handler can observe the `-1`.
        // SAFETY: restoring the disposition captured at install time.
        unsafe {
            let _ = sigaction(Signal::SIGWINCH, &self.previous);
        }
        SIGWINCH_PIPE_W.store(-1, Ordering::SeqCst);
    }
}
