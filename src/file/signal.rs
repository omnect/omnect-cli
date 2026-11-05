use log::warn;
use signal_hook::{
    consts::{SIGHUP, SIGINT, SIGTERM},
    iterator::Signals,
    low_level,
};
use std::{
    collections::BTreeSet,
    env,
    ffi::c_int,
    fs::{self, Permissions},
    io,
    mem::MaybeUninit,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process, ptr,
    sync::{Mutex, MutexGuard, Once, PoisonError},
    thread,
};
use tempfile::{Builder, TempDir};

const STOP_SIGNALS: [c_int; 3] = [SIGHUP, SIGINT, SIGTERM];
// exit code of a process that a signal stopped, plus the signal number
const SIGNAL_EXIT_CODE_BASE: c_int = 128;
const DEFAULT_PREFIX: &str = ".tmp";
// the dirs hold images and keys, so other users must not read them
const DIR_MODE: u32 = 0o700;

static DIRS: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());
static HANDLER: Once = Once::new();

/// A temp dir that is also removed if a stop signal ends the process.
pub struct RegisteredTempDir(TempDir);

impl RegisteredTempDir {
    /// Creates the dir in TMPDIR.
    pub fn create() -> io::Result<Self> {
        Self::create_in(&env::temp_dir(), DEFAULT_PREFIX)
    }

    /// Creates the dir in `parent`, with a name that starts with `prefix`.
    pub fn create_in(parent: &Path, prefix: &str) -> io::Result<Self> {
        let dir = Builder::new()
            .prefix(prefix)
            .permissions(Permissions::from_mode(DIR_MODE))
            .tempdir_in(parent)?;
        register(dir.path());
        Ok(Self(dir))
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }

    /// Keeps the dir if a stop signal ends the process. Drop still removes it.
    pub fn unregister(&self) {
        unregister(self.path());
    }

    /// Removes the dir again if a stop signal ends the process.
    pub fn register(&self) {
        register(self.path());
    }

    /// Keeps the dir, also on drop.
    pub fn keep(&mut self) {
        self.unregister();
        self.0.disable_cleanup(true);
    }
}

impl Drop for RegisteredTempDir {
    fn drop(&mut self) {
        self.unregister();
    }
}

fn register(dir: &Path) {
    HANDLER.call_once(start_handler);
    dirs().insert(dir.to_path_buf());
}

fn unregister(dir: &Path) {
    dirs().remove(dir);
}

#[cfg(test)]
pub(crate) fn is_registered(dir: &Path) -> bool {
    dirs().contains(dir)
}

fn dirs() -> MutexGuard<'static, BTreeSet<PathBuf>> {
    DIRS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn start_handler() {
    // e.g. `nohup` ignores SIGHUP, and a shell ignores SIGINT for a background job
    let signals: Vec<_> = STOP_SIGNALS
        .into_iter()
        .filter(|signal| !is_ignored(*signal))
        .collect();
    let mut signals = match Signals::new(signals) {
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

fn is_ignored(signal: c_int) -> bool {
    let mut action = MaybeUninit::<libc::sigaction>::zeroed();
    // SAFETY: without a new action, sigaction only writes the current one into `action`
    let result = unsafe { libc::sigaction(signal, ptr::null(), action.as_mut_ptr()) };
    // SAFETY: the action was zeroed, and sigaction fills it on success
    result == 0 && unsafe { action.assume_init() }.sa_sigaction == libc::SIG_IGN
}

#[cfg(test)]
mod tests {
    use crate::file::signal::*;

    const MODE_MASK: u32 = 0o777;

    #[test]
    fn registered_temp_dir_is_registered_until_drop() {
        let dir = RegisteredTempDir::create().expect("create temp dir");
        let path = dir.path().to_path_buf();

        assert!(is_registered(&path));
        drop(dir);
        assert!(!is_registered(&path));
        assert!(!path.exists());
    }

    #[test]
    fn registered_temp_dir_is_private() {
        let dir = RegisteredTempDir::create().expect("create temp dir");

        let mode = fs::metadata(dir.path())
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & MODE_MASK, DIR_MODE);
    }

    #[test]
    fn registered_temp_dir_register_again_after_unregister() {
        let dir = RegisteredTempDir::create().expect("create temp dir");

        dir.unregister();
        assert!(!is_registered(dir.path()));
        dir.register();
        assert!(is_registered(dir.path()));
    }

    #[test]
    fn registered_temp_dir_keep_unregisters_and_keeps_the_dir() {
        let mut dir = RegisteredTempDir::create().expect("create temp dir");
        let path = dir.path().to_path_buf();

        dir.keep();
        assert!(!is_registered(&path));
        drop(dir);
        assert!(path.exists());
        fs::remove_dir(&path).expect("remove dir");
    }

    #[test]
    fn is_ignored_reads_the_disposition() {
        // SAFETY: SIGUSR2 is not used by the tests
        unsafe { libc::signal(libc::SIGUSR2, libc::SIG_IGN) };

        assert!(is_ignored(libc::SIGUSR2));
        assert!(!is_ignored(libc::SIGUSR1));
    }
}
