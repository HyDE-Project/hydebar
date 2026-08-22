//! Supervised spawning, and the guard that ends a group on cancellation.

use std::{io, process::Output};

use log::{info, warn};
use tokio::process::{Child, Command};

use super::{
    marks::{LAUNCH_VAR, SPAWN_VAR, launch_id, next_spawn_id, terminate_detached},
    registry::LIVE_GROUPS,
    termination::terminate_group
};

/// Puts the child, and everything it spawns, in a process group of its own.
///
/// The group identifier equals the child's own identifier, which is what makes
/// [`terminate_group`] able to reach every descendant.
pub fn in_own_group(command: &mut Command) -> &mut Command {
    command.process_group(0)
}

/// Prepares a command whose process must not outlive the bar.
///
/// The child gets a process group of its own, so [`terminate_group`] reaches
/// every descendant, and is stamped with the launch id, so a later bar can
/// recognise what an earlier one left behind.
///
/// The kernel offers a third guard — a signal delivered to the child once its
/// parent is gone — and it is deliberately not used. That signal is armed
/// against the *thread* that performed the spawn rather than against the
/// process, and the bar spawns from runtime threads that come and go: a pool
/// thread retiring while the bar runs on killed perfectly healthy modules and
/// took their icons off the bar with them. It is also dropped the moment the
/// child forks, so it never reached the helpers a listener starts, which are
/// the processes that actually pile up.
///
/// What replaces it is [`claim_orphans`]: a listener that outlives the task
/// watching it comes back to the bar instead of being adopted away, so the
/// registry and the sweep still see it.
pub fn supervised(command: &mut Command) -> &mut Command {
    in_own_group(command).env(LAUNCH_VAR, launch_id())
}

/// Makes the bar the parent every orphaned descendant returns to.
///
/// Without this a listener whose shell exits hands its helpers to the service
/// manager, which has no idea they belong to a bar and never ends them; they
/// then outlive every mechanism here, because nothing can reach a process it
/// cannot find. Claiming them keeps the whole family inside the bar's own
/// subtree for as long as the bar runs, which is what makes [`sweep_orphans`]
/// and the termination handler complete rather than best effort.
///
/// # Errors
///
/// Returns the failure reported by the operating system, which on a kernel too
/// old to know the request means the bar keeps the behaviour it had before.
pub fn claim_orphans() -> io::Result<()> {
    #[expect(
        unsafe_code,
        reason = "the request carries an integer argument and reads no memory the caller owns"
    )]
    let claimed = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) };

    if claimed != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

/// Starts `command` supervised and guarded against cancellation.
///
/// The guard is built from the identifier the spawn just returned, with no
/// await point in between, so there is no window in which the task can be
/// dropped while the group is already running but nothing is watching it.
///
/// # Errors
///
/// Returns the spawn failure reported by the operating system.
pub fn spawn_guarded(command: &mut Command) -> io::Result<(Child, Option<GroupGuard>)> {
    let spawn = next_spawn_id();
    let child = supervised(command).env(SPAWN_VAR, &spawn).spawn()?;
    let guard = child
        .id()
        .map(|pid| GroupGuard::new(pid).following_detached(spawn));

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
    pub(super) pid: Option<u32>,
    /// Spawn stamp to sweep for once the group has been ended.
    spawn:          Option<String>
}

impl GroupGuard {
    /// Guards the group led by `pid`.
    #[must_use]
    pub fn new(pid: u32) -> Self {
        if !LIVE_GROUPS.insert(pid) {
            warn!(
                "the process group registry is full, the group led by {pid} will only be ended by \
                 its guard"
            );
        }

        Self {
            pid:   Some(pid),
            spawn: None
        }
    }

    /// Extends the guard to the descendants that leave the group behind.
    ///
    /// `spawn` is the value of [`SPAWN_VAR`] the command was started with.
    #[must_use]
    pub fn following_detached(mut self, spawn: String) -> Self {
        self.spawn = Some(spawn);

        self
    }

    /// Lets the group outlive this guard.
    ///
    /// Called once the child has been reaped: the identifier is free to be
    /// handed to an unrelated process from that moment on, and signalling it
    /// later would hit a stranger. A command that ran to its own end has also
    /// had the chance to take its helpers down, so nothing is swept.
    pub fn release(&mut self) {
        self.spawn = None;

        if let Some(pid) = self.pid.take() {
            LIVE_GROUPS.remove(pid);
        }
    }
}

impl Drop for GroupGuard {
    fn drop(&mut self) {
        if let Some(pid) = self.pid.take() {
            LIVE_GROUPS.remove(pid);
            let _ = terminate_group(pid);

            if let Some(spawn) = self.spawn.take() {
                let ended = terminate_detached(&spawn);

                if ended > 0 {
                    info!("ended {ended} processes detached from a cancelled command");
                }
            }
        }
    }
}
