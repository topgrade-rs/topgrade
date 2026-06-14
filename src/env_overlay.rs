use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
use std::sync::MutexGuard;

type EnvOverlay = BTreeMap<OsString, OsString>;

static ENV: OnceLock<Mutex<EnvOverlay>> = OnceLock::new();

#[cfg(test)]
static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env() -> &'static Mutex<EnvOverlay> {
    ENV.get_or_init(|| Mutex::new(EnvOverlay::new()))
}

#[cfg(any(unix, test))]
pub fn replace(vars: BTreeMap<OsString, OsString>) {
    *env().lock().expect("env overlay lock poisoned") = vars;
}

fn snapshot() -> EnvOverlay {
    env().lock().expect("env overlay lock poisoned").clone()
}

pub fn path() -> Option<OsString> {
    env()
        .lock()
        .expect("env overlay lock poisoned")
        .get(OsStr::new("PATH"))
        .cloned()
}

pub fn apply_to_command(command: &mut Command) {
    let explicit_envs = command
        .get_envs()
        .map(|(key, _)| key.to_os_string())
        .collect::<BTreeSet<_>>();

    for (key, value) in snapshot() {
        if explicit_envs.contains(&key) {
            continue;
        }
        command.env(key, value);
    }
}

#[cfg(test)]
pub fn test_guard() -> EnvOverlayTestGuard {
    let test_guard = TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env overlay test lock poisoned");
    let original = snapshot();
    replace(EnvOverlay::new());
    EnvOverlayTestGuard {
        _test_guard: test_guard,
        original,
    }
}

#[cfg(test)]
pub struct EnvOverlayTestGuard {
    _test_guard: MutexGuard<'static, ()>,
    original: EnvOverlay,
}

#[cfg(test)]
impl Drop for EnvOverlayTestGuard {
    fn drop(&mut self) {
        replace(self.original.clone());
    }
}
