//! Single instance ownership of the bar.
//!
//! Starting hydebar while another copy is already drawing must not leave two
//! bars on screen: the newcomer takes over and the incumbent goes away. The
//! hand over is arbitrated by a lock file under `$XDG_RUNTIME_DIR/hydebar/`,
//! falling back to `/tmp/hydebar-$UID/` when the compositor session exports no
//! runtime directory.
//!
//! The identity is the user, not the configuration file: a bar owns the layer
//! surfaces of every requested output, so a second bar started with another
//! configuration would fight the first one for the same screen real estate
//! rather than complement it. Running two configurations side by side is
//! therefore deliberately not supported, and `--config-path` only selects which
//! configuration the single instance reads.
//!
//! Ownership is held by an advisory `flock` on the lock file, whose payload is
//! the process id of the owner. The kernel drops the lock when the owning
//! process dies, so a crashed instance never blocks startup; the recorded
//! process id is only used to signal the incumbent and to describe it in error
//! messages.

use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, Instant}
};

use log::{debug, info, warn};

/// Name of the lock file inside the runtime directory.
const LOCK_FILE_NAME: &str = "instance.lock";

/// How long a newcomer waits for the incumbent to release the lock.
const TAKEOVER_TIMEOUT: Duration = Duration::from_millis(2000);

/// How often the newcomer retries the lock while waiting.
const TAKEOVER_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Reason the process could not become the single instance.
#[derive(Debug)]
pub enum InstanceError {
    /// The runtime directory holding the lock file could not be prepared.
    Directory(PathBuf, io::Error),
    /// The lock file could not be opened.
    Open(PathBuf, io::Error),
    /// Locking the file failed for a reason other than contention.
    Lock(PathBuf, io::Error),
    /// The owner process could not be signalled to quit.
    Signal(i32, io::Error),
    /// The previous instance still held the lock when the wait ran out.
    Timeout(Option<i32>, Duration)
}

impl fmt::Display for InstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Directory(path, err) => {
                write!(f, "failed to create the runtime directory {path:?}: {err}")
            }
            Self::Open(path, err) => write!(f, "failed to open the lock file {path:?}: {err}"),
            Self::Lock(path, err) => write!(f, "failed to lock {path:?}: {err}"),
            Self::Signal(pid, err) => {
                write!(f, "failed to ask the running instance {pid} to quit: {err}")
            }
            Self::Timeout(Some(pid), waited) => write!(
                f,
                "the running instance {pid} did not quit within {} ms, refusing to draw a second \
                 bar",
                waited.as_millis()
            ),
            Self::Timeout(None, waited) => write!(
                f,
                "another instance kept the lock for {} ms, refusing to draw a second bar",
                waited.as_millis()
            )
        }
    }
}

impl std::error::Error for InstanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Directory(_, err) | Self::Open(_, err) | Self::Lock(_, err) => Some(err),
            Self::Signal(_, err) => Some(err),
            Self::Timeout(..) => None
        }
    }
}

/// Ownership of the single instance slot.
///
/// The lock lives as long as this value: dropping it, or the process exiting
/// for any reason, hands the slot to the next starting bar.
#[derive(Debug)]
pub struct InstanceLock {
    file: File,
    path: PathBuf
}

impl InstanceLock {
    /// Path of the lock file this ownership is recorded in.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes the current process id into the lock file.
    fn record_owner(&mut self) -> io::Result<()> {
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        write!(self.file, "{}", std::process::id())?;
        self.file.flush()
    }
}

/// How long a takeover waits, and how often it retries meanwhile.
#[derive(Debug, Clone, Copy)]
pub struct TakeoverPolicy {
    /// Upper bound on the wait for the incumbent to release the lock.
    pub timeout:       Duration,
    /// Delay between two attempts at the lock while waiting.
    pub poll_interval: Duration
}

impl Default for TakeoverPolicy {
    fn default() -> Self {
        Self {
            timeout:       TAKEOVER_TIMEOUT,
            poll_interval: TAKEOVER_POLL_INTERVAL
        }
    }
}

/// Directory the lock file is created in.
fn lock_dir() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir).join("hydebar"),
        _ => PathBuf::from(format!("/tmp/hydebar-{}", current_uid()))
    }
}

/// User id the fallback runtime directory is named after.
fn current_uid() -> u32 {
    unsafe { libc::getuid() }
}

/// Path of the lock file arbitrating the single instance slot.
pub fn lock_path() -> PathBuf {
    lock_dir().join(LOCK_FILE_NAME)
}

/// Opens the lock file, creating the runtime directory when needed.
fn open_lock_file(path: &Path) -> Result<File, InstanceError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| InstanceError::Directory(parent.to_path_buf(), err))?;
    }

    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)
        .map_err(|err| InstanceError::Open(path.to_path_buf(), err))
}

/// Takes the lock without blocking, reporting `false` when it is held.
fn try_flock(file: &File) -> io::Result<bool> {
    let outcome = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };

    if outcome == 0 {
        return Ok(true);
    }

    let err = io::Error::last_os_error();

    match err.raw_os_error() {
        Some(code) if code == libc::EWOULDBLOCK || code == libc::EINTR => Ok(false),
        _ => Err(err)
    }
}

/// Claims the slot when it is free, reporting `None` while another live
/// instance holds it.
///
/// A lock file left behind by a crashed instance is free as far as the kernel
/// is concerned, so this is also the path that reclaims a stale lock.
fn try_acquire(path: &Path) -> Result<Option<InstanceLock>, InstanceError> {
    let file = open_lock_file(path)?;

    if !try_flock(&file).map_err(|err| InstanceError::Lock(path.to_path_buf(), err))? {
        return Ok(None);
    }

    let mut lock = InstanceLock {
        file,
        path: path.to_path_buf()
    };
    lock.record_owner()
        .map_err(|err| InstanceError::Open(path.to_path_buf(), err))?;

    Ok(Some(lock))
}

/// Reads the process id recorded in the lock file, if it holds a readable one.
fn read_owner(path: &Path) -> Option<i32> {
    let mut contents = String::new();
    File::open(path).ok()?.read_to_string(&mut contents).ok()?;

    contents.trim().parse::<i32>().ok()
}

/// Reports whether a process with this id still exists.
fn process_is_alive(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }

    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }

    io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Asks the process to quit, letting it destroy its surfaces on the way out.
fn request_quit(pid: i32) -> io::Result<()> {
    if unsafe { libc::kill(pid, libc::SIGTERM) } == 0 {
        return Ok(());
    }

    let err = io::Error::last_os_error();

    if err.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }

    Err(err)
}

/// Becomes the single instance, displacing a running bar if there is one.
///
/// The slot is free, or left behind by a crashed instance, in the common case
/// and is taken straight away. Otherwise the incumbent is asked to quit and
/// given [`TakeoverPolicy::timeout`] to release the lock; a bar that ignores
/// the request aborts startup rather than adding a second bar to the screen.
pub fn acquire() -> Result<InstanceLock, InstanceError> {
    acquire_at(&lock_path(), TakeoverPolicy::default(), request_quit)
}

/// [`acquire`] against an explicit path, wait policy and signalling routine.
fn acquire_at<S>(
    path: &Path,
    policy: TakeoverPolicy,
    signal_owner: S
) -> Result<InstanceLock, InstanceError>
where
    S: Fn(i32) -> io::Result<()>
{
    if let Some(lock) = try_acquire(path)? {
        debug!("took the instance lock at {path:?}");

        return Ok(lock);
    }

    let owner = read_owner(path).filter(|pid| process_is_alive(*pid));

    match owner {
        Some(pid) => {
            info!("another hydebar instance ({pid}) is running, asking it to quit");
            signal_owner(pid).map_err(|err| InstanceError::Signal(pid, err))?;
        }
        None => warn!("the instance lock at {path:?} is held by an unidentified process")
    }

    wait_for_takeover(path, policy, owner)
}

/// Retries the lock until the incumbent releases it or the wait runs out.
fn wait_for_takeover(
    path: &Path,
    policy: TakeoverPolicy,
    owner: Option<i32>
) -> Result<InstanceLock, InstanceError> {
    let started = Instant::now();

    loop {
        sleep(policy.poll_interval);

        if let Some(lock) = try_acquire(path)? {
            info!(
                "previous instance released the lock after {:?}",
                started.elapsed()
            );

            return Ok(lock);
        }

        if started.elapsed() >= policy.timeout {
            return Err(InstanceError::Timeout(owner, policy.timeout));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        process::Command,
        sync::{
            Mutex,
            atomic::{AtomicU32, Ordering}
        }
    };

    use super::*;

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Serialises the tests that take and release a lock.
    ///
    /// The lock is a file the whole process shares through the kernel, so two
    /// of these running at once see each other's state and fail for reasons
    /// that have nothing to do with what they assert.
    static SERIAL: Mutex<()> = Mutex::new(());

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("hydebar-instance-{}-{unique}", std::process::id()));
            fs::create_dir_all(&path).expect("failed to create the test directory");

            Self(path)
        }

        fn lock_path(&self) -> PathBuf {
            self.0.join(LOCK_FILE_NAME)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fast_policy() -> TakeoverPolicy {
        TakeoverPolicy {
            timeout:       Duration::from_millis(150),
            poll_interval: Duration::from_millis(5)
        }
    }

    fn never_signalled(_: i32) -> io::Result<()> {
        panic!("the takeover path must not signal anyone when the slot is free");
    }

    // a process id that is guaranteed to be gone: the child is reaped before
    // the id is read back
    fn dead_pid() -> i32 {
        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg("exit 0")
            .spawn()
            .expect("failed to spawn the throwaway process");
        let pid = child.id() as i32;
        child.wait().expect("failed to reap the throwaway process");

        pid
    }

    #[test]
    fn an_unheld_lock_is_acquired() {
        let _serial = SERIAL.lock().unwrap_or_else(|err| err.into_inner());
        let dir = TempDir::new();
        let path = dir.lock_path();

        let lock = acquire_at(&path, fast_policy(), never_signalled)
            .expect("an unheld lock must be acquired");

        assert_eq!(lock.path(), path);
        assert_eq!(read_owner(&path), Some(std::process::id() as i32));
    }

    #[test]
    fn a_lock_held_by_a_live_process_is_detected() {
        let _serial = SERIAL.lock().unwrap_or_else(|err| err.into_inner());
        let dir = TempDir::new();
        let path = dir.lock_path();

        let _held = try_acquire(&path)
            .expect("the first attempt must succeed")
            .expect("the slot starts out free");

        assert!(
            try_acquire(&path)
                .expect("probing a held lock is not an error")
                .is_none()
        );

        let owner = read_owner(&path).expect("the holder records its process id");
        assert_eq!(owner, std::process::id() as i32);
        assert!(process_is_alive(owner));
    }

    #[test]
    fn a_stale_lock_whose_process_is_gone_is_taken_over() {
        let _serial = SERIAL.lock().unwrap_or_else(|err| err.into_inner());
        let dir = TempDir::new();
        let path = dir.lock_path();
        let stale = dead_pid();
        fs::write(&path, stale.to_string()).expect("failed to plant the stale lock file");

        assert!(!process_is_alive(stale));

        let lock = acquire_at(&path, fast_policy(), never_signalled)
            .expect("a stale lock must not block startup");

        assert_eq!(lock.path(), path);
        assert_eq!(read_owner(&path), Some(std::process::id() as i32));
    }

    #[test]
    fn the_takeover_signals_the_owner_and_waits_for_the_lock() {
        let _serial = SERIAL.lock().unwrap_or_else(|err| err.into_inner());
        let dir = TempDir::new();
        let path = dir.lock_path();

        let held = Mutex::new(Some(
            try_acquire(&path)
                .expect("the first attempt must succeed")
                .expect("the slot starts out free")
        ));
        let signalled = Mutex::new(Vec::new());

        let lock = acquire_at(&path, fast_policy(), |pid| {
            signalled.lock().expect("signal log poisoned").push(pid);
            held.lock().expect("held lock poisoned").take();

            Ok(())
        })
        .expect("a cooperating instance hands the slot over");

        assert_eq!(
            signalled.into_inner().expect("signal log poisoned"),
            vec![std::process::id() as i32]
        );
        assert_eq!(lock.path(), path);
        assert_eq!(read_owner(&path), Some(std::process::id() as i32));
    }

    #[test]
    fn an_owner_that_never_quits_aborts_the_takeover() {
        let _serial = SERIAL.lock().unwrap_or_else(|err| err.into_inner());
        let dir = TempDir::new();
        let path = dir.lock_path();

        let _held = try_acquire(&path)
            .expect("the first attempt must succeed")
            .expect("the slot starts out free");

        let policy = fast_policy();
        let started = Instant::now();
        let err = acquire_at(&path, policy, |_| Ok(())).expect_err("the slot is never released");

        assert!(started.elapsed() >= policy.timeout);
        assert!(
            matches!(err, InstanceError::Timeout(Some(pid), _) if pid == std::process::id() as i32)
        );
    }

    #[test]
    fn releasing_the_lock_frees_the_slot() {
        let _serial = SERIAL.lock().unwrap_or_else(|err| err.into_inner());
        let dir = TempDir::new();
        let path = dir.lock_path();

        let lock = try_acquire(&path)
            .expect("the first attempt must succeed")
            .expect("the slot starts out free");
        drop(lock);

        assert!(
            try_acquire(&path)
                .expect("the slot is free again")
                .is_some()
        );
    }

    #[test]
    fn the_lock_lives_in_the_runtime_directory() {
        let path = lock_path();

        assert!(path.ends_with(LOCK_FILE_NAME));
        assert!(
            path.parent()
                .is_some_and(|parent| parent.ends_with("hydebar")
                    || parent.starts_with(format!("/tmp/hydebar-{}", current_uid())))
        );
    }
}
