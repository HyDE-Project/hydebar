//! The lock file and its non-blocking acquisition.
//!
//! The file lives under `$XDG_RUNTIME_DIR/hydebar/`, falling back to
//! `/tmp/hydebar-$UID/` when the compositor session exports no runtime
//! directory. Ownership is an advisory `flock` held for as long as the
//! [`InstanceLock`] value lives; the payload of the file is the owner's
//! process id, recorded so a newcomer can name and signal the incumbent.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Seek, SeekFrom, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf}
};

use super::error::InstanceError;

/// Name of the lock file inside the runtime directory.
pub(super) const LOCK_FILE_NAME: &str = "instance.lock";

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

/// Directory the lock file is created in.
fn lock_dir() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir).join("hydebar"),
        _ => PathBuf::from(format!("/tmp/hydebar-{}", current_uid()))
    }
}

/// User id the fallback runtime directory is named after.
pub(super) fn current_uid() -> u32 {
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
pub(super) fn try_acquire(path: &Path) -> Result<Option<InstanceLock>, InstanceError> {
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
