//! Child processes that die together with the bar.
//!
//! A listener is a shell loop that spawns a helper every few seconds. Killing
//! only the shell leaves the loop's children behind, and every restart of the
//! bar adds another crop of them: a thousand of them heat a machine as
//! effectively as a stress test. Putting each listener in its own process group
//! makes it possible to end the whole family with one signal.
//!
//! Two ways out of the bar have to end those groups. A module whose task is
//! aborted — a configuration reload, a schedule change — is covered by
//! [`GroupGuard`], which fires while the cancelled future is dropped. A
//! termination signal is not: the bar answers it by removing its surfaces and
//! leaving through [`std::process::exit`], which runs no destructor at all.
//! Every guarded group is therefore also recorded in a process wide registry
//! that [`terminate_all`] walks on the way out.

use std::{
    collections::HashSet,
    io,
    process::Output,
    sync::{Mutex, OnceLock}
};

use log::warn;
use tokio::process::{Child, Command};

/// Leaders of the groups the bar started and has not ended yet.
type Registry = Mutex<HashSet<u32>>;

/// Registry every [`GroupGuard`] records itself in.
fn live_groups() -> &'static Registry {
    static GROUPS: OnceLock<Registry> = OnceLock::new();

    GROUPS.get_or_init(Registry::default)
}

/// Reads or edits `registry`, treating a poisoned lock as usable.
///
/// A panic while the set was borrowed says nothing about the process ids in it,
/// and refusing to read them afterwards would strand exactly the children this
/// module exists to reap.
fn with_registry<R>(registry: &Registry, edit: impl FnOnce(&mut HashSet<u32>) -> R) -> R {
    let mut groups = registry.lock().unwrap_or_else(|err| err.into_inner());

    edit(&mut groups)
}

/// Puts the child, and everything it spawns, in a process group of its own.
///
/// The group identifier equals the child's own identifier, which is what makes
/// [`terminate_group`] able to reach every descendant.
pub fn in_own_group(command: &mut Command) -> &mut Command {
    command.process_group(0)
}

/// Starts `command` in a group of its own, guarded against cancellation.
///
/// The guard is built from the identifier the spawn just returned, with no
/// await point in between, so there is no window in which the task can be
/// dropped while the group is already running but nothing is watching it.
///
/// # Errors
///
/// Returns the spawn failure reported by the operating system.
pub fn spawn_guarded(command: &mut Command) -> io::Result<(Child, Option<GroupGuard>)> {
    let child = in_own_group(command).spawn()?;
    let guard = child.id().map(GroupGuard::new);

    Ok((child, guard))
}

/// Runs `command` to completion and collects whatever it printed.
///
/// [`Command::output`] abandons the child when the future driving it is
/// dropped, so a scheduled command that is still running when its module is
/// reloaded survives the reload, and so does everything the command started.
/// Holding a guard across the wait makes cancellation reach the whole group.
/// The standard streams are left to the caller: only the ones it piped are
/// collected.
///
/// # Errors
///
/// Returns the spawn or wait failure reported by the operating system.
pub async fn guarded_output(command: &mut Command) -> io::Result<Output> {
    let (child, mut guard) = spawn_guarded(command)?;
    let output = child.wait_with_output().await?;

    if let Some(guard) = guard.as_mut() {
        guard.release();
    }

    Ok(output)
}

/// Ends the whole process group led by `pid`.
///
/// Sends the polite signal first and the final one after, since a shell loop
/// blocked in `sleep` ignores the first until the sleep is over.
pub fn terminate_group(pid: u32) -> io::Result<()> {
    let group = -(pid as i32);

    signal(group, libc::SIGTERM)?;
    signal(group, libc::SIGKILL)
}

/// Ends every process group the bar still owns.
///
/// This is the last thing the bar does before [`std::process::exit`]: a
/// takeover asks the running instance to quit with `SIGTERM`, and the outgoing
/// bar never unwinds, so no [`GroupGuard`] would otherwise run. Without this
/// call every restart of the bar leaves its listener shells behind, and they
/// keep spawning helpers forever.
pub fn terminate_all() {
    terminate_registered(live_groups());
}

/// Ends every group recorded in `registry`, emptying it.
fn terminate_registered(registry: &Registry) {
    let leaders: Vec<u32> = with_registry(registry, |groups| groups.drain().collect());

    for pid in leaders {
        if let Err(err) = terminate_group(pid) {
            warn!("failed to end the process group led by {pid}: {err}");
        }
    }
}

/// Sends `signal` to the process group `group`, reporting failure.
fn signal(group: i32, signal: i32) -> io::Result<()> {
    let sent = unsafe { libc::kill(group, signal) };

    if sent == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Ends a process group when it goes out of scope.
///
/// A listener task can be aborted at any moment, and an aborted future simply
/// stops: without this the shell loop it started would keep running, and every
/// reload of the configuration would leave another one behind. The group is
/// also recorded in the process wide registry, so a bar that exits without
/// unwinding still takes it along.
#[derive(Debug)]
pub struct GroupGuard {
    /// Leader of the group, absent once it has been released.
    pid: Option<u32>
}

impl GroupGuard {
    /// Guards the group led by `pid`.
    #[must_use]
    pub fn new(pid: u32) -> Self {
        with_registry(live_groups(), |groups| groups.insert(pid));

        Self {
            pid: Some(pid)
        }
    }

    /// Lets the group outlive this guard.
    ///
    /// Called once the child has been reaped: the identifier is free to be
    /// handed to an unrelated process from that moment on, and signalling it
    /// later would hit a stranger.
    pub fn release(&mut self) {
        if let Some(pid) = self.pid.take() {
            with_registry(live_groups(), |groups| groups.remove(&pid));
        }
    }
}

impl Drop for GroupGuard {
    fn drop(&mut self) {
        if let Some(pid) = self.pid.take() {
            with_registry(live_groups(), |groups| groups.remove(&pid));
            let _ = terminate_group(pid);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{process::Stdio, time::Duration};

    use super::*;

    /// A process id far beyond `pid_max`, so signalling it reaches nobody.
    const UNUSED_PID: u32 = u32::MAX / 2;

    /// Starts a shell loop that keeps a helper of its own alive.
    fn spawn_loop() -> tokio::process::Child {
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg("while :; do sleep 30 & wait; done")
            .stdout(Stdio::null());

        in_own_group(&mut command).spawn().expect("listener")
    }

    #[tokio::test]
    async fn a_listener_and_its_children_die_together() {
        let mut child = spawn_loop();
        let pid = child.id().expect("a running listener");

        tokio::time::sleep(Duration::from_millis(200)).await;

        terminate_group(pid).expect("the group ends");

        let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .expect("the listener stops in time");

        assert!(status.is_ok());
    }

    #[tokio::test]
    async fn dropping_the_guard_ends_the_group() {
        let mut child = spawn_loop();
        let pid = child.id().expect("a running listener");

        {
            let _guard = GroupGuard::new(pid);
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .expect("the listener stops in time");

        assert!(status.is_ok());
    }

    #[test]
    fn a_released_guard_leaves_the_group_alone() {
        let mut guard = GroupGuard::new(UNUSED_PID);

        guard.release();

        assert!(guard.pid.is_none());
    }

    #[test]
    fn a_guard_is_recorded_until_it_goes_away() {
        let recorded = |pid: u32| with_registry(live_groups(), |groups| groups.contains(&pid));

        let guard = GroupGuard::new(UNUSED_PID - 1);
        assert!(recorded(UNUSED_PID - 1));

        drop(guard);
        assert!(!recorded(UNUSED_PID - 1));
    }

    #[test]
    fn a_released_guard_is_no_longer_recorded() {
        let mut guard = GroupGuard::new(UNUSED_PID - 2);

        guard.release();

        assert!(!with_registry(live_groups(), |groups| groups.contains(&(UNUSED_PID - 2))));
    }

    #[tokio::test]
    async fn the_exit_path_ends_every_group_it_recorded() {
        let registry = Registry::default();

        let mut first = spawn_loop();
        let mut second = spawn_loop();
        let first_pid = first.id().expect("a running listener");
        let second_pid = second.id().expect("a running listener");

        with_registry(&registry, |groups| {
            groups.insert(first_pid);
            groups.insert(second_pid);
        });

        tokio::time::sleep(Duration::from_millis(200)).await;

        terminate_registered(&registry);

        for child in [&mut first, &mut second] {
            tokio::time::timeout(Duration::from_secs(5), child.wait())
                .await
                .expect("the listener stops in time")
                .expect("the listener is reaped");
        }

        assert!(with_registry(&registry, |groups| groups.is_empty()));
    }

    #[tokio::test]
    async fn a_guarded_run_reports_what_the_command_printed() {
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg("printf hello")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = guarded_output(&mut command)
            .await
            .expect("the command runs");

        assert_eq!(output.stdout, b"hello");
    }

    #[test]
    fn ending_a_group_that_is_already_gone_is_not_a_failure() {
        assert!(terminate_group(UNUSED_PID).is_ok());
    }
}
