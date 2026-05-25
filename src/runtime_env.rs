use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
use std::sync::MutexGuard;

type RuntimeEnv = BTreeMap<OsString, OsString>;

static ENV: OnceLock<Mutex<RuntimeEnv>> = OnceLock::new();

#[cfg(test)]
static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env() -> &'static Mutex<RuntimeEnv> {
    ENV.get_or_init(|| Mutex::new(RuntimeEnv::new()))
}

pub fn replace(vars: RuntimeEnv) {
    *env().lock().expect("runtime env lock poisoned") = vars;
}

pub fn snapshot() -> RuntimeEnv {
    env().lock().expect("runtime env lock poisoned").clone()
}

pub fn path() -> Option<OsString> {
    env()
        .lock()
        .expect("runtime env lock poisoned")
        .get(OsStr::new("PATH"))
        .cloned()
}

#[cfg(test)]
pub fn test_guard() -> RuntimeEnvTestGuard {
    let test_guard = TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("runtime env test lock poisoned");
    let original = snapshot();
    replace(RuntimeEnv::new());
    RuntimeEnvTestGuard {
        _test_guard: test_guard,
        original,
    }
}

#[cfg(test)]
pub struct RuntimeEnvTestGuard {
    _test_guard: MutexGuard<'static, ()>,
    original: RuntimeEnv,
}

#[cfg(test)]
impl Drop for RuntimeEnvTestGuard {
    fn drop(&mut self) {
        replace(self.original.clone());
    }
}
