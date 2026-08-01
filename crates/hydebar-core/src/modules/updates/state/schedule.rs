//! The one runner behind every update check, and its cadence.

use std::{sync::Arc, time::Duration};

use log::error;
use tokio::{runtime::Handle, sync::Notify, task::JoinHandle, time::sleep};

use super::{
    HydeSnapshot, Message,
    failures::{FailureLog, report},
    super::commands::{self, CheckFailure}
};
use crate::{ModuleEventSender, config::UpdatesModuleConfig};

/// Shortest interval a configuration is allowed to ask for.
///
/// Every check spawns a package manager that talks to a mirror. A
/// configuration asking for one a second would be answered by the mirror
/// rather than by the bar, so the floor is enforced here instead of
/// trusting the file.
pub(super) const MIN_INTERVAL: Duration = Duration::from_mins(1);

/// Longest a single check may run before it is given up on.
///
/// A package manager waiting on an unreachable mirror can hang for as long
/// as its own timeouts allow, and the schedule has only one runner:
/// without a deadline of its own a single stuck check would silence the
/// module until the bar restarts.
const CHECK_TIMEOUT: Duration = Duration::from_mins(5);

/// The one task that ever runs the check command.
///
/// It outlives every configuration reload that leaves the check unchanged,
/// and it is the only place a check is started from: the manual button
/// wakes it rather than spawning a second runner, so two checks can
/// never overlap and a reload can never leave a half-finished one
/// behind.
pub(super) struct Schedule {
    /// Check command this schedule was started for.
    command:  Arc<str>,
    /// Time between the end of one check and the start of the next.
    interval: Duration,
    /// `HyDE` branch the schedule compares the clone against.
    branch:   Arc<str>,
    /// Wake-up the manual button rings.
    wake:     Arc<Notify>,
    /// The running task, aborted when this schedule is dropped.
    pub(super) handle: JoinHandle<()>
}

impl std::fmt::Debug for Schedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Schedule")
            .field("command", &self.command)
            .field("interval", &self.interval)
            .finish()
    }
}

impl Schedule {
    /// Starts the check loop on `runtime`.
    pub(super) fn start(
        runtime: &Handle,
        sender: ModuleEventSender<Message>,
        command: Arc<str>,
        interval: Duration,
        hyde_clone: Option<Arc<str>>,
        branch: Arc<str>
    ) -> Self {
        let wake = Arc::new(Notify::new());
        let loop_wake = Arc::clone(&wake);
        let loop_command = Arc::clone(&command);
        let loop_branch = Arc::clone(&branch);

        let handle = runtime.spawn(check_loop(
            sender,
            loop_command,
            interval,
            loop_wake,
            hyde_clone,
            loop_branch
        ));

        Self {
            command,
            interval,
            branch,
            wake,
            handle
        }
    }

    /// Reports whether this schedule already does what is being asked for.
    pub(super) fn matches(&self, command: &str, interval: Duration, branch: &str) -> bool {
        self.command.as_ref() == command
            && self.interval == interval
            && self.branch.as_ref() == branch
    }

    /// Asks for a check as soon as the runner is free.
    ///
    /// Repeated requests collapse into one, and a request arriving during a
    /// check is answered once that check is done rather than beside it.
    pub(super) fn request_check(&self) {
        self.wake.notify_one();
    }
}

impl Drop for Schedule {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// Runs the check command forever, once per interval and on request.
///
/// A `HyDE` clone, when one is known, is compared against upstream on the
/// same cadence and by the same single runner, so the two checks can
/// never race each other over the network.
async fn check_loop(
    sender: ModuleEventSender<Message>,
    command: Arc<str>,
    interval: Duration,
    wake: Arc<Notify>,
    hyde_clone: Option<Arc<str>>,
    branch: Arc<str>
) {
    let mut failures = FailureLog::default();
    let mut hyde_failures = FailureLog::default();

    loop {
        let outcome = check_once(command.as_ref(), &mut failures).await;

        if let Err(err) = sender.try_send(outcome) {
            error!("failed to publish the updates check result: {err}");
        }

        if let Some(clone) = hyde_clone.as_deref()
            && let Some(snapshot) =
                check_hyde_once(clone, branch.as_ref(), &mut hyde_failures).await
            && let Err(err) = sender.try_send(Message::HydeChecked(snapshot))
        {
            error!("failed to publish the hyde check result: {err}");
        }

        tokio::select! {
            () = sleep(interval) => {}
            () = wake.notified() => {}
        }
    }
}

/// Compares the `HyDE` clone against upstream once.
///
/// A failure keeps the last snapshot standing: an unreachable forge is no
/// reason to tell the user their desktop stopped existing.
async fn check_hyde_once(
    clone: &str,
    branch: &str,
    failures: &mut FailureLog
) -> Option<HydeSnapshot> {
    match tokio::time::timeout(CHECK_TIMEOUT, commands::check_hyde(clone, branch)).await {
        Ok(Ok((version, commits))) => {
            failures.clear();

            Some(HydeSnapshot {
                version,
                commits
            })
        }
        Ok(Err(err)) => {
            report(failures, format!("the hyde check failed: {err}"));

            None
        }
        Err(_) => {
            report(
                failures,
                format!("the hyde check did not finish within {CHECK_TIMEOUT:?}")
            );

            None
        }
    }
}

/// Runs the check command once and turns whatever happened into a message.
///
/// A failure never leaves the loop: the module reports what it can and the
/// next interval tries again.
async fn check_once(command: &str, failures: &mut FailureLog) -> Message {
    match tokio::time::timeout(CHECK_TIMEOUT, commands::check_for_updates(command)).await {
        Ok(Ok(updates)) => {
            failures.clear();

            Message::UpdatesCheckCompleted(updates)
        }
        Ok(Err(CheckFailure::Unavailable(err))) => {
            report(failures, format!("the check command cannot be run: {err}"));

            Message::UpdatesUnavailable
        }
        Ok(Err(CheckFailure::Transient(err))) => {
            report(failures, format!("the check command failed: {err}"));

            Message::CheckFailed
        }
        Err(_) => {
            report(
                failures,
                format!("the check command did not finish within {CHECK_TIMEOUT:?}")
            );

            Message::CheckFailed
        }
    }
}

/// Interval the configuration asks for, never below the floor.
pub(super) fn check_interval(config: &UpdatesModuleConfig) -> Duration {
    Duration::from_secs(config.check_interval).max(MIN_INTERVAL)
}
