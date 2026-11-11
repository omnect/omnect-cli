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
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::{Path, PathBuf},
    process, ptr,
    sync::{Mutex, MutexGuard, Once, PoisonError},
    thread,
};
use tempfile::{Builder, TempDir};

const STOP_SIGNALS: [c_int; 3] = [SIGHUP, SIGINT, SIGTERM];
const INIT_PID: u32 = 1;
// exit code of a process that a signal stopped, plus the signal number
const SIGNAL_EXIT_CODE_BASE: c_int = 128;
const DEFAULT_PREFIX: &str = ".tmp";
// the dirs hold images and keys, so other users must not read them
const DIR_MODE: u32 = 0o700;

static DIRS: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());
static HANDLER: Once = Once::new();

/// A temp dir that is also removed if a stop signal ends the process.
pub struct RegisteredTempDir {
    dir: TempDir,
    kept: bool,
}

impl RegisteredTempDir {
    /// Creates the dir in TMPDIR.
    pub fn create() -> io::Result<Self> {
        let tmp_dir = env::temp_dir();
        ensure_no_colon(&tmp_dir).map_err(io::Error::other)?;
        Self::create_in(&tmp_dir, DEFAULT_PREFIX)
    }

    /// Creates the dir in `parent`, with a name that starts with `prefix`.
    pub fn create_in(parent: &Path, prefix: &str) -> io::Result<Self> {
        let mut dir = Builder::new()
            .prefix(prefix)
            .permissions(Permissions::from_mode(DIR_MODE))
            .tempdir_in(parent)?;
        // removed by Drop
        dir.disable_cleanup(true);
        register(dir.path());
        Ok(Self { dir, kept: false })
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
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
        self.kept = true;
    }

    pub fn is_kept(&self) -> bool {
        self.kept
    }
}

impl Drop for RegisteredTempDir {
    /// Removes the dir before it unregisters it, so a stop signal during the removal of a large
    /// file still removes the rest.
    fn drop(&mut self) {
        if !self.kept
            && let Err(e) = fs::remove_dir_all(self.path())
            && e.kind() != io::ErrorKind::NotFound
        {
            warn!("cannot remove {}: {e}", self.path().display());
        }
        self.unregister();
    }
}

/// Starts the handler that removes the registered dirs on a stop signal. As PID 1, e.g. in a
/// container, the process ignores a stop signal for which no handler is installed.
pub fn handle_stop_signals() {
    HANDLER.call_once(start_handler);
}

/// The partition files are in TMPDIR, and e2tools split their `<file>:<path>` argument at the
/// first `:`.
pub fn ensure_tmp_dir_has_no_colon() -> anyhow::Result<()> {
    ensure_no_colon(&env::temp_dir())
}

fn ensure_no_colon(tmp_dir: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(
        !tmp_dir.as_os_str().as_bytes().contains(&b':'),
        "TMPDIR {} contains ':', which e2tools cannot read; set TMPDIR to a path without ':'",
        tmp_dir.display()
    );
    Ok(())
}

fn register(dir: &Path) {
    handle_stop_signals();
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
            // as PID 1, e.g. in a container, the process ignores a signal it raises itself, and
            // the abort after it ends in SIGSEGV
            if process::id() == INIT_PID {
                process::exit(SIGNAL_EXIT_CODE_BASE + signal);
            }
            // the caller sees the signal as the cause of the stop; for a stop signal this does
            // not return, it aborts if the signal cannot be raised
            let _ = low_level::emulate_default_handler(signal);
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

        assert!(!dir.is_kept());
        dir.keep();
        assert!(dir.is_kept());
        assert!(!is_registered(&path));
        drop(dir);
        assert!(path.exists());
        fs::remove_dir(&path).expect("remove dir");
    }

    #[test]
    fn ensure_no_colon_rejects_a_tmpdir_with_a_colon() {
        ensure_no_colon(Path::new("/data/ci-job1")).expect("no colon");
        let error = ensure_no_colon(Path::new("/data/ci:job1")).expect_err("colon");
        assert!(error.to_string().contains("set TMPDIR"), "{error}");
    }

    #[test]
    fn is_ignored_reads_the_disposition() {
        // SAFETY: SIGUSR2 is not used by the other tests
        let old = unsafe { libc::signal(libc::SIGUSR2, libc::SIG_IGN) };
        let ignored = is_ignored(libc::SIGUSR2);
        // SAFETY: restores the disposition read above
        unsafe { libc::signal(libc::SIGUSR2, old) };

        assert!(ignored);
        assert!(!is_ignored(libc::SIGUSR1));
    }
}
