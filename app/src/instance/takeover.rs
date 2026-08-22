//! Displacing a running bar and waiting for it to hand the slot over.
//!
//! When the slot is taken by a live incumbent, the newcomer asks it to quit
//! with `SIGTERM` — letting it destroy its surfaces on the way out — and then
//! retries the lock on the cadence of a [`TakeoverPolicy`]. A bar that
//! ignores the request exhausts the wait and aborts startup rather than
//! adding a second bar to the screen.

use std::{
    io,
    path::Path,
    thread::sleep,
    time::{Duration, Instant}
};

use log::{debug, info, warn};

use super::{
    error::InstanceError,
    lock::{InstanceLock, lock_path, try_acquire},
    stale::{process_is_alive, read_owner}
};

/// How long a newcomer waits for the incumbent to release the lock.
const TAKEOVER_TIMEOUT: Duration = Duration::from_secs(2);

/// How often the newcomer retries the lock while waiting.
const TAKEOVER_POLL_INTERVAL: Duration = Duration::from_millis(25);

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

/// Asks the process to quit, letting it destroy its surfaces on the way out.
fn request_quit(pid: i32) -> io::Result<()> {
    #[expect(
        unsafe_code,
        reason = "delivers one signal to an id read from the lock file; no caller memory is touched"
    )]
    let sent = unsafe { libc::kill(pid, libc::SIGTERM) };

    if sent == 0 {
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
pub(super) fn acquire_at<S>(
    path: &Path,
    policy: TakeoverPolicy,
    signal_owner: S
) -> Result<InstanceLock, InstanceError>
where
    S: Fn(i32) -> io::Result<()>
{
    if let Some(lock) = try_acquire(path)? {
        debug!("took the instance lock at {}", path.display());

        return Ok(lock);
    }

    let owner = read_owner(path).filter(|pid| process_is_alive(*pid));

    match owner {
        Some(pid) => {
            info!("another hydebar instance ({pid}) is running, asking it to quit");
            signal_owner(pid).map_err(|err| InstanceError::Signal(pid, err))?;
        }
        None => warn!(
            "the instance lock at {} is held by an unidentified process",
            path.display()
        )
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
