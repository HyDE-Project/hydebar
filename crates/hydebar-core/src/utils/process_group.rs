//! Child processes that die together with the bar.
//!
//! A listener is a shell loop that spawns a helper every few seconds. Killing
//! only the shell leaves the loop's children behind, and every restart of the
//! bar adds another crop of them: a few hundred of them heat a machine as
//! effectively as a stress test. Putting each listener in its own process group
//! makes it possible to end the whole family with one signal.
//!
//! Nothing may be trusted to run on the way out, so the guarantee is built from
//! three independent layers.
//!
//! * [`GroupGuard`] ends a group while the cancelled future that owns it is
//!   dropped, which covers a configuration reload or a schedule change.
//! * [`install_termination_handler`] covers a bar that is signalled rather than
//!   asked: the handler ends every recorded group with nothing but atomic loads
//!   and `kill`, both of which a signal handler may use, and then wakes a
//!   thread that ends the process once the graceful path has had its chance.
//! * [`claim_orphans`] keeps a listener whose shell has gone inside the bar's
//!   own subtree instead of letting the service manager adopt it, so the two
//!   layers above still have something they can reach.
//!
//! A bar that still finds strays from an earlier run — one started before this
//! machinery existed, or one left by a bar that was killed outright — sweeps
//! them up at startup through [`sweep_orphans`]. Recognition is by the launch
//! stamp [`LAUNCH_VAR`] carries into every supervised process and, through the
//! inherited environment, into everything those processes start; no command
//! text is matched, so a foreign bar or an unrelated shell loop is never
//! touched.

mod guard;
mod marks;
mod reaper;
mod registry;
mod termination;

pub use guard::{
    GroupGuard, claim_orphans, guarded_output, in_own_group, spawn_guarded, supervised
};
pub use marks::{LAUNCH_VAR, SPAWN_VAR, launch_id, sweep_orphans};
pub use reaper::start_orphan_reaper;
pub use termination::{install_termination_handler, terminate_all, terminate_group};

#[cfg(test)]
mod tests {
    use std::{
        fs,
        process::Stdio,
        sync::atomic::{AtomicUsize, Ordering},
        thread,
        time::{Duration, Instant}
    };

    use tokio::process::Command;

    use super::{
        marks::{
            LAUNCH_PREFIX, SPAWN_PREFIX, marked_processes, marked_stamp, next_spawn_id,
            terminate_marked
        },
        registry::{GroupRegistry, LIVE_GROUPS, REGISTRY_CAPACITY},
        *
    };

    /// A process id far beyond `pid_max`, so signalling it reaches nobody.
    const UNUSED_PID: u32 = u32::MAX / 2;

    /// Distinguishes the launch stamps two tests plant at the same time.
    static STAMPS: AtomicUsize = AtomicUsize::new(0);

    /// A launch stamp no other test and no running bar can be wearing.
    fn unique_stamp() -> String {
        let ordinal = STAMPS.fetch_add(1, Ordering::Relaxed);

        format!("test-{}-{ordinal}", std::process::id())
    }

    /// Starts a shell loop that keeps a helper of its own alive.
    fn spawn_loop() -> tokio::process::Child {
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg("while :; do sleep 30 & wait; done")
            .stdout(Stdio::null());

        in_own_group(&mut command).spawn().expect("listener")
    }

    /// Starts a shell loop wearing `stamp` as its launch id.
    fn spawn_marked_loop(stamp: &str) -> tokio::process::Child {
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg("while :; do sleep 30 & wait; done")
            .env(LAUNCH_VAR, stamp)
            .stdout(Stdio::null());

        in_own_group(&mut command).spawn().expect("listener")
    }

    /// Reports whether `pid` is still a live, unreaped process.
    fn is_running(pid: u32) -> bool {
        let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) else {
            return false;
        };

        stat.rsplit(')')
            .next()
            .and_then(|rest| rest.split_whitespace().next())
            .is_some_and(|state| state != "Z")
    }

    /// Waits for `pid` to stop running, reporting whether it did in time.
    fn wait_until_gone(pid: u32) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);

        while Instant::now() < deadline {
            if !is_running(pid) {
                return true;
            }

            thread::sleep(Duration::from_millis(25));
        }

        false
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
        let guard = GroupGuard::new(UNUSED_PID - 1);
        assert!(LIVE_GROUPS.contains(UNUSED_PID - 1));

        drop(guard);
        assert!(!LIVE_GROUPS.contains(UNUSED_PID - 1));
    }

    #[test]
    fn a_released_guard_is_no_longer_recorded() {
        let mut guard = GroupGuard::new(UNUSED_PID - 2);

        guard.release();

        assert!(!LIVE_GROUPS.contains(UNUSED_PID - 2));
    }

    #[test]
    fn the_registry_holds_a_group_until_it_is_removed() {
        let registry = GroupRegistry::new();

        assert!(registry.insert(41));
        assert!(registry.contains(41));

        registry.remove(41);

        assert!(!registry.contains(41));
    }

    #[test]
    fn the_registry_reports_when_it_has_no_room_left() {
        let registry = GroupRegistry::new();

        let capacity = u32::try_from(REGISTRY_CAPACITY).expect("small capacity");

        for pid in 1..=capacity {
            assert!(registry.insert(pid), "slot {pid} must be free");
        }

        assert!(!registry.insert(capacity + 1));
    }

    #[tokio::test]
    async fn the_exit_path_ends_every_group_it_recorded() {
        let registry = GroupRegistry::new();

        let mut first = spawn_loop();
        let mut second = spawn_loop();
        let first_pid = first.id().expect("a running listener");
        let second_pid = second.id().expect("a running listener");

        registry.insert(first_pid);
        registry.insert(second_pid);

        tokio::time::sleep(Duration::from_millis(200)).await;

        assert_eq!(registry.terminate_all(), 2);

        for child in [&mut first, &mut second] {
            tokio::time::timeout(Duration::from_secs(5), child.wait())
                .await
                .expect("the listener stops in time")
                .expect("the listener is reaped");
        }

        assert!(!registry.contains(first_pid));
        assert!(!registry.contains(second_pid));
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

    #[test]
    fn the_launch_stamp_is_the_same_for_every_reader() {
        assert_eq!(launch_id(), launch_id());
        assert!(launch_id().starts_with(&format!("{}-", std::process::id())));
    }

    #[test]
    fn the_marker_prefix_matches_the_variable_it_stands_for() {
        assert_eq!(LAUNCH_PREFIX, format!("{LAUNCH_VAR}=").as_bytes());
        assert_eq!(SPAWN_PREFIX, format!("{SPAWN_VAR}=").as_bytes());
    }

    #[test]
    fn no_two_spawns_share_a_stamp() {
        let first = next_spawn_id();
        let second = next_spawn_id();

        assert_ne!(first, second);
        assert!(first.starts_with(launch_id()));
        assert!(second.starts_with(launch_id()));
    }

    #[tokio::test]
    async fn a_guarded_command_carries_a_spawn_stamp_of_its_own() {
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg("printf %s \"$HYDEBAR_SPAWN_ID\"")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let first = guarded_output(&mut command)
            .await
            .expect("the command runs");
        let second = guarded_output(&mut command)
            .await
            .expect("the command runs again");

        assert!(!first.stdout.is_empty());
        assert_ne!(first.stdout, second.stdout);
    }

    /// What a cancelled update check leaves behind: `fakeroot` starts `faked`
    /// in a session of its own, so ending the group reaches the shell and not
    /// the daemon. The guard has to follow the stamp to it.
    #[tokio::test]
    async fn a_cancelled_command_takes_its_detached_helper_along() {
        let stamp = unique_stamp();

        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg("setsid sleep 60 >/dev/null 2>&1 & sleep 60")
            .env(SPAWN_VAR, &stamp)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let mut child = supervised(&mut command).spawn().expect("the command runs");
        let pid = child.id().expect("a running command");
        let guard = GroupGuard::new(pid).following_detached(stamp.clone());

        tokio::time::sleep(Duration::from_millis(300)).await;

        let marked = marked_processes(SPAWN_PREFIX, &|found: &str| found == stamp);
        assert!(
            marked.len() >= 2,
            "the helper left the group but kept the stamp: {marked:?}"
        );

        drop(guard);

        let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;

        for pid in marked {
            assert!(wait_until_gone(pid), "{pid} outlived the cancelled command");
        }
    }

    #[test]
    fn a_stamped_environment_reveals_the_run_that_started_it() {
        let environ = b"PATH=/usr/bin\0HYDEBAR_LAUNCH_ID=7-42\0HOME=/home/user\0";

        assert_eq!(marked_stamp(environ, LAUNCH_PREFIX), Some("7-42"));
    }

    #[test]
    fn an_environment_without_the_stamp_reveals_nothing() {
        let environ = b"PATH=/usr/bin\0WAYBAR_SOMETHING=1\0";

        assert_eq!(marked_stamp(environ, LAUNCH_PREFIX), None);
    }

    /// The stamp is a whole entry, never a fragment of a longer name: a
    /// variable ending in the marker's name must not be mistaken for it.
    #[test]
    fn a_lookalike_variable_is_not_the_stamp() {
        let environ = b"NOT_HYDEBAR_LAUNCH_ID=7-42\0";

        assert_eq!(marked_stamp(environ, LAUNCH_PREFIX), None);
    }

    #[tokio::test]
    async fn a_supervised_command_carries_the_launch_stamp() {
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg("printf %s \"$HYDEBAR_LAUNCH_ID\"")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = guarded_output(&mut command)
            .await
            .expect("the command runs");

        assert_eq!(String::from_utf8_lossy(&output.stdout), launch_id());
    }

    #[tokio::test]
    async fn the_sweep_ends_the_strays_of_another_run() {
        let stamp = unique_stamp();
        let mut stray = spawn_marked_loop(&stamp);
        let pid = stray.id().expect("a running listener");

        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(marked_processes(LAUNCH_PREFIX, &|found: &str| found == stamp).contains(&pid));

        let ended = terminate_marked(LAUNCH_PREFIX, |found| found == stamp);

        assert!(ended >= 1);
        tokio::time::timeout(Duration::from_secs(5), stray.wait())
            .await
            .expect("the stray stops in time")
            .expect("the stray is reaped");
    }

    #[tokio::test]
    async fn the_sweep_leaves_the_processes_of_another_stamp_alone() {
        let stamp = unique_stamp();
        let hunted = unique_stamp();
        let mut mine = spawn_marked_loop(&stamp);
        let pid = mine.id().expect("a running listener");

        tokio::time::sleep(Duration::from_millis(200)).await;

        assert_eq!(terminate_marked(LAUNCH_PREFIX, |found| found == hunted), 0);
        assert!(is_running(pid));

        let _ = terminate_group(pid);
        let _ = mine.wait().await;
    }

    /// A group kill cannot reach a descendant that started a session of its
    /// own and outlived the parent that started it; the stamp still can, which
    /// is what the exit path relies on to catch the last stragglers.
    #[tokio::test]
    async fn a_detached_descendant_is_still_recognised_by_its_stamp() {
        let stamp = unique_stamp();
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg("setsid sleep 60 >/dev/null 2>&1 & printf ready")
            .env(LAUNCH_VAR, &stamp)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = in_own_group(&mut command)
            .output()
            .await
            .expect("the command runs");
        assert_eq!(output.stdout, b"ready");

        let detached = marked_processes(LAUNCH_PREFIX, &|found: &str| found == stamp);
        assert_eq!(detached.len(), 1, "the session of its own survived alone");

        assert!(terminate_marked(LAUNCH_PREFIX, |found| found == stamp) >= 1);
        assert!(wait_until_gone(detached[0]));
    }

    /// An unmarked shell loop belongs to somebody else and must survive a
    /// sweep that accepts every stamp it finds.
    #[tokio::test]
    async fn the_sweep_never_reaches_an_unmarked_process() {
        let mut stranger = spawn_loop();
        let pid = stranger.id().expect("a running listener");

        tokio::time::sleep(Duration::from_millis(200)).await;

        assert!(!marked_processes(LAUNCH_PREFIX, &|_: &str| true).contains(&pid));
        assert!(is_running(pid));

        let _ = terminate_group(pid);
        let _ = stranger.wait().await;
    }

    /// A supervised module keeps running when the thread that started it goes.
    ///
    /// This is the failure a kernel level death signal introduced: armed
    /// against the spawning thread rather than the process, it killed healthy
    /// modules whenever a runtime worker retired, and the bar lost its icons.
    #[tokio::test]
    async fn a_supervised_child_outlives_the_thread_that_started_it() {
        let handle = tokio::runtime::Handle::current();
        let mut child = thread::spawn(move || {
            let _entered = handle.enter();
            let mut command = Command::new("bash");
            command
                .arg("-c")
                .arg("while :; do sleep 30 & wait; done")
                .stdout(Stdio::null());

            supervised(&mut command).spawn().expect("listener")
        })
        .join()
        .expect("the spawning thread finishes");

        let pid = child.id().expect("a running listener");
        tokio::time::sleep(Duration::from_millis(200)).await;

        assert!(
            is_running(pid),
            "the module died with the thread that started it"
        );

        let _ = terminate_group(pid);
        let _ = child.wait().await;
    }
}
