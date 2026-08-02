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
//!
//! The lock file itself lives in [`lock`], the reading of a possibly stale
//! owner in [`stale`], the displacement of a running bar in [`takeover`] and
//! the failure vocabulary in [`error`].

mod error;
mod lock;
mod stale;
mod takeover;

pub use error::InstanceError;
pub use takeover::acquire;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{
        fs, io,
        path::PathBuf,
        process::Command,
        sync::{
            Mutex,
            atomic::{AtomicU32, Ordering}
        },
        time::{Duration, Instant}
    };

    use super::{
        InstanceError,
        lock::{InstanceLock, LOCK_FILE_NAME, current_uid, lock_path, try_acquire},
        stale::{process_is_alive, read_owner},
        takeover::{TakeoverPolicy, acquire_at}
    };

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
        let pid = child.id().cast_signed();
        child.wait().expect("failed to reap the throwaway process");

        pid
    }

    #[test]
    fn an_unheld_lock_is_acquired() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = TempDir::new();
        let path = dir.lock_path();

        let lock: InstanceLock = acquire_at(&path, fast_policy(), never_signalled)
            .expect("an unheld lock must be acquired");

        assert_eq!(lock.path(), path);
        assert_eq!(read_owner(&path), Some(std::process::id().cast_signed()));
    }

    #[test]
    fn a_lock_held_by_a_live_process_is_detected() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        assert_eq!(owner, std::process::id().cast_signed());
        assert!(process_is_alive(owner));
    }

    #[test]
    fn a_stale_lock_whose_process_is_gone_is_taken_over() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = TempDir::new();
        let path = dir.lock_path();
        let stale = dead_pid();
        fs::write(&path, stale.to_string()).expect("failed to plant the stale lock file");

        assert!(!process_is_alive(stale));

        let lock = acquire_at(&path, fast_policy(), never_signalled)
            .expect("a stale lock must not block startup");

        assert_eq!(lock.path(), path);
        assert_eq!(read_owner(&path), Some(std::process::id().cast_signed()));
    }

    #[test]
    fn the_takeover_signals_the_owner_and_waits_for_the_lock() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
            vec![std::process::id().cast_signed()]
        );
        assert_eq!(lock.path(), path);
        assert_eq!(read_owner(&path), Some(std::process::id().cast_signed()));
    }

    #[test]
    fn an_owner_that_never_quits_aborts_the_takeover() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
            matches!(err, InstanceError::Timeout(Some(pid), _) if pid == std::process::id().cast_signed())
        );
    }

    #[test]
    fn releasing_the_lock_frees_the_slot() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
