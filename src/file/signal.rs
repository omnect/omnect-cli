use log::warn;
use signal_hook::{
    consts::{SIGHUP, SIGINT, SIGTERM},
    iterator::Signals,
    low_level,
};
use std::{
    collections::BTreeSet,
    ffi::c_int,
    fs,
    path::{Path, PathBuf},
    process,
    sync::{Mutex, MutexGuard, Once, PoisonError},
    thread,
};

const STOP_SIGNALS: [c_int; 3] = [SIGHUP, SIGINT, SIGTERM];
// exit code of a process that a signal stopped, plus the signal number
const SIGNAL_EXIT_CODE_BASE: c_int = 128;

static DIRS: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());
static HANDLER: Once = Once::new();

/// Removes `dir` if a stop signal ends the process before `unregister`. A dir next to the
/// output image, which can hold a whole image, is not left there by e.g. Ctrl-C.
pub fn register(dir: &Path) {
    HANDLER.call_once(start_handler);
    dirs().insert(dir.to_path_buf());
}

pub fn unregister(dir: &Path) {
    dirs().remove(dir);
}

#[cfg(test)]
pub fn is_registered(dir: &Path) -> bool {
    dirs().contains(dir)
}

fn dirs() -> MutexGuard<'static, BTreeSet<PathBuf>> {
    DIRS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn start_handler() {
    let mut signals = match Signals::new(STOP_SIGNALS) {
        Ok(signals) => signals,
        Err(e) => {
            warn!("cannot handle stop signals, so a stop leaves temp dirs: {e}");
            return;
        }
    };
    thread::spawn(move || {
        if let Some(signal) = signals.forever().next() {
            // the lock is held until the process ends, so no new dir is registered
            let dirs = dirs();
            for dir in dirs.iter() {
                if let Err(e) = fs::remove_dir_all(dir) {
                    warn!("cannot remove {}: {e}", dir.display());
                }
            }
            // the caller sees the signal as the cause of the stop
            if let Err(e) = low_level::emulate_default_handler(signal) {
                warn!("cannot stop with signal {signal}: {e}");
            }
            process::exit(SIGNAL_EXIT_CODE_BASE + signal);
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::file::signal::*;

    #[test]
    fn register_and_unregister() {
        let dir = tempfile::tempdir().expect("create temp dir");

        register(dir.path());
        assert!(is_registered(dir.path()));
        unregister(dir.path());
        assert!(!is_registered(dir.path()));
    }
}
