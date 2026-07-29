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

use std::{
    fs, io,
    process::Output,
    ptr,
    sync::{
        OnceLock,
        atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering}
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH}
};

use log::{info, warn};
use tokio::process::{Child, Command};

/// Environment variable stamping every process the bar supervises.
///
/// Its value identifies the run that started the process, which is what lets a
/// later bar tell its own children from the strays of a previous one.
pub const LAUNCH_VAR: &str = "HYDEBAR_LAUNCH_ID";

/// [`LAUNCH_VAR`] as it appears in a `/proc/<pid>/environ` entry.
const LAUNCH_PREFIX: &[u8] = b"HYDEBAR_LAUNCH_ID=";

/// Environment variable stamping one supervised spawn.
///
/// The launch stamp tells this bar's processes from an earlier bar's, which is
/// too coarse to cancel a single command: ending everything carrying it would
/// take every other listener down as well. This one narrows the same trick to
/// one spawn, so a cancelled command can be followed to the descendants that
/// left its process group behind.
pub const SPAWN_VAR: &str = "HYDEBAR_SPAWN_ID";

/// [`SPAWN_VAR`] as it appears in a `/proc/<pid>/environ` entry.
const SPAWN_PREFIX: &[u8] = b"HYDEBAR_SPAWN_ID=";

/// Serial number handed to the next supervised spawn.
static SPAWN_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Process groups the registry can hold at once.
///
/// One slot per live listener or scheduled command; a configuration with a
/// module per slot would be unusable long before the registry filled up.
const REGISTRY_CAPACITY: usize = 512;

/// Value stored in a registry slot that holds no group.
const EMPTY_SLOT: u32 = 0;

/// Time the graceful exit path is given before the process is ended anyway.
///
/// The groups are already gone by then, so this only decides how long a bar
/// whose event loop stopped answering keeps its surfaces on screen.
const HARD_EXIT_GRACE: Duration = Duration::from_millis(1000);

/// Groups the bar started and has not ended yet.
///
/// The registry is a fixed array of atomics rather than a locked set because a
/// termination signal has to be able to walk it: taking a lock in a signal
/// handler risks deadlocking against the thread that already holds it, and a
/// deadlock here is exactly the leak this module exists to prevent.
struct GroupRegistry {
    /// Leader of one group per occupied slot.
    slots: [AtomicU32; REGISTRY_CAPACITY]
}

impl GroupRegistry {
    /// A registry holding no group.
    const fn new() -> Self {
        Self {
            slots: [const { AtomicU32::new(EMPTY_SLOT) }; REGISTRY_CAPACITY]
        }
    }

    /// Records the group led by `pid`, reporting whether a slot was free.
    fn insert(&self, pid: u32) -> bool {
        self.slots.iter().any(|slot| {
            slot.compare_exchange(EMPTY_SLOT, pid, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
        })
    }

    /// Forgets the group led by `pid`.
    fn remove(&self, pid: u32) {
        for slot in &self.slots {
            if slot
                .compare_exchange(pid, EMPTY_SLOT, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
        }
    }

    /// Reports whether the group led by `pid` is recorded.
    ///
    /// Only the tests observe the registry this way; live code either records
    /// or ends groups and never has to ask.
    #[cfg(test)]
    fn contains(&self, pid: u32) -> bool {
        self.slots
            .iter()
            .any(|slot| slot.load(Ordering::Acquire) == pid)
    }

    /// Ends every recorded group, emptying the registry.
    ///
    /// Only an atomic swap and `kill` are used, so this is safe to run from a
    /// signal handler; it reports how many groups it reached rather than
    /// logging, because logging from a handler is not.
    fn terminate_all(&self) -> usize {
        let mut ended = 0;

        for slot in &self.slots {
            let pid = slot.swap(EMPTY_SLOT, Ordering::AcqRel);

            if pid != EMPTY_SLOT {
                kill_group(pid);
                ended += 1;
            }
        }

        ended
    }
}

/// Registry every [`GroupGuard`] records itself in.
static LIVE_GROUPS: GroupRegistry = GroupRegistry::new();

/// Write end of the pipe a termination signal wakes the reaper thread through.
///
/// Negative while no handler is installed.
static WAKE_WRITER: AtomicI32 = AtomicI32::new(-1);

/// Stamp identifying this run of the bar.
///
/// The process id alone would not do: identifiers are reused, and a stray that
/// happens to carry the id of the bar reading it would be mistaken for one of
/// its own children. Pairing it with the moment the bar started makes such a
/// collision impossible in practice.
pub fn launch_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();

    ID.get_or_init(|| {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());

        format!("{}-{started}", std::process::id())
    })
}

/// Stamp identifying one spawn among every spawn of every run.
///
/// Built on top of [`launch_id`] so that two bars running side by side cannot
/// hand out the same value and sweep each other's descendants.
fn next_spawn_id() -> String {
    let serial = SPAWN_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{}-{serial}", launch_id())
}

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
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) } != 0 {
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

/// Ends the whole process group led by `pid`.
///
/// Sends the polite signal first and the final one after, since a shell loop
/// blocked in `sleep` ignores the first until the sleep is over.
pub fn terminate_group(pid: u32) -> io::Result<()> {
    let group = -(pid as i32);

    signal(group, libc::SIGTERM)?;
    signal(group, libc::SIGKILL)
}

/// [`terminate_group`] without a report, for the paths that cannot make one.
fn kill_group(pid: u32) {
    let group = -(pid as i32);

    unsafe {
        libc::kill(group, libc::SIGTERM);
        libc::kill(group, libc::SIGKILL);
    }
}

/// Ends every process group the bar still owns, then every straggler.
///
/// This is the last thing the bar does before [`std::process::exit`], which
/// runs no destructor and would otherwise leave every listener shell behind.
///
/// The groups go first, so nothing keeps starting new work while the second
/// half runs. That second half exists because a group kill cannot reach a
/// descendant that detached itself: `faked`, which the update check leaves
/// behind through `fakeroot`, forks twice and lets its parent exit, ending up
/// outside every group the bar knows about. The launch stamp survives the
/// detachment, because the environment does, so those stragglers are still
/// recognisable as this run's.
pub fn terminate_all() {
    let ended = LIVE_GROUPS.terminate_all();

    if ended > 0 {
        info!("ended {ended} process groups on the way out");
    }

    let own = launch_id();
    let strays = terminate_marked(LAUNCH_PREFIX, |stamp| stamp == own);

    if strays > 0 {
        info!("ended {strays} detached processes on the way out");
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

/// Signals that ask the bar to stop and are answered by a reaping handler.
///
/// The rest — `SIGKILL` above all — cannot be caught, which is what the parent
/// death signal armed by [`supervised`] is for.
const TERMINATION_SIGNALS: [i32; 3] = [libc::SIGTERM, libc::SIGINT, libc::SIGHUP];

/// Makes a termination signal end every group the bar started.
///
/// The bar answers `SIGTERM` through its event loop, which needs the loop to
/// still be running and the runtime to still be scheduling; when either has
/// stopped, the process dies with its listeners intact and they keep spawning
/// helpers on a timer forever. The handler installed here does not depend on
/// either: it walks the registry with atomic loads and ends each group with
/// `kill`, both of which a signal handler is allowed to do, and then wakes a
/// thread that ends the process should the graceful path fail to.
///
/// Registration composes with the runtime's own signal handling rather than
/// replacing it, so the orderly shutdown — taking the surfaces off screen
/// before exiting — still happens whenever the event loop is alive to do it.
///
/// # Errors
///
/// Returns the failure reported when the wake pipe, the reaper thread or a
/// handler registration could not be set up.
pub fn install_termination_handler() -> io::Result<()> {
    let reader = open_wake_pipe()?;

    thread::Builder::new()
        .name("hydebar-reaper".to_owned())
        .spawn(move || await_termination(reader))?;

    for number in TERMINATION_SIGNALS {
        unsafe {
            signal_hook_registry::register(number, on_termination_signal)?;
        }
    }

    Ok(())
}

/// Creates the wake pipe, publishing its write end, and returns the read end.
fn open_wake_pipe() -> io::Result<i32> {
    let mut ends = [0; 2];

    if unsafe { libc::pipe2(ends.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }

    WAKE_WRITER.store(ends[1], Ordering::Release);

    Ok(ends[0])
}

/// Ends every group and wakes the reaper, using only what a handler may call.
fn on_termination_signal() {
    LIVE_GROUPS.terminate_all();

    let writer = WAKE_WRITER.load(Ordering::Acquire);

    if writer >= 0 {
        let wake = 1u8;

        unsafe {
            libc::write(writer, ptr::addr_of!(wake).cast(), 1);
        }
    }
}

/// Ends the process once a termination signal has been reaped.
///
/// The graceful path is given its moment first: it removes the surfaces and
/// exits on its own, and this thread only matters when it no longer can.
fn await_termination(reader: i32) {
    if !wait_for_wake(reader) {
        return;
    }

    info!("termination signal reaped, ending the process");
    thread::sleep(HARD_EXIT_GRACE);
    terminate_all();
    std::process::exit(0);
}

/// Blocks until the handler writes, reporting whether a wake arrived.
fn wait_for_wake(reader: i32) -> bool {
    let mut wake = 0u8;

    loop {
        let read = unsafe { libc::read(reader, ptr::addr_of_mut!(wake).cast(), 1) };

        if read == 1 {
            return true;
        }

        if read < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
            continue;
        }

        return false;
    }
}

/// Ends the supervised processes an earlier bar left behind.
///
/// Returns how many were ended. A bar that crashed before its listeners could
/// be reaped leaves shell loops that keep spawning helpers on a timer; picking
/// them up here means a restart is enough to clean a machine up, and that the
/// count of supervised processes does not grow from one run to the next.
pub fn sweep_orphans() -> usize {
    let own = launch_id();
    let ended = terminate_marked(LAUNCH_PREFIX, |stamp| stamp != own);

    if ended > 0 {
        warn!("ended {ended} processes left behind by an earlier run");
    }

    ended
}

/// Ends the descendants of one spawn that left its process group.
///
/// A group kill cannot reach a process that called `setsid`, and the helpers
/// that do so are exactly the ones a cancelled command cannot clean up itself:
/// `fakeroot` starts its `faked` daemon in a session of its own and only ends
/// it from an exit trap, which a signalled shell never runs. The spawn stamp
/// rides into `faked` through the inherited environment, so the daemon stays
/// recognisable as belonging to this one command and to nothing else.
fn terminate_detached(spawn: &str) -> usize {
    terminate_marked(SPAWN_PREFIX, |stamp| stamp == spawn)
}

/// Ends every process whose stamp under `prefix` is accepted by `wanted`.
///
/// Two passes, because the first one may land between a shell loop starting a
/// helper and the scan noticing it; the helper carries the same stamp through
/// the inherited environment, so the second pass finds it.
fn terminate_marked(prefix: &[u8], wanted: impl Fn(&str) -> bool) -> usize {
    let mut ended = 0;

    for _ in 0..2 {
        for pid in marked_processes(prefix, &wanted) {
            if unsafe { libc::kill(pid as i32, libc::SIGKILL) } == 0 {
                ended += 1;
            }
        }
    }

    ended
}

/// Lists the running processes whose stamp under `prefix` `wanted` accepts.
fn marked_processes(prefix: &[u8], wanted: &impl Fn(&str) -> bool) -> Vec<u32> {
    let own = std::process::id();
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| *pid != own)
        .filter(|pid| {
            fs::read(format!("/proc/{pid}/environ"))
                .ok()
                .and_then(|environ| marked_stamp(&environ, prefix).map(wanted))
                .unwrap_or(false)
        })
        .collect()
}

/// Reads the stamp carried under `prefix` out of a NUL separated environment.
fn marked_stamp<'a>(environ: &'a [u8], prefix: &[u8]) -> Option<&'a str> {
    environ.split(|byte| *byte == 0).find_map(|entry| {
        let stamp = entry.strip_prefix(prefix)?;

        std::str::from_utf8(stamp).ok()
    })
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
    pid:   Option<u32>,
    /// Spawn stamp to sweep for once the group has been ended.
    spawn: Option<String>
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

#[cfg(test)]
mod tests {
    use std::{
        process::Stdio,
        sync::atomic::AtomicUsize,
        time::{Duration, Instant}
    };

    use super::*;

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

        for pid in 1..=REGISTRY_CAPACITY {
            assert!(registry.insert(pid as u32), "slot {pid} must be free");
        }

        assert!(!registry.insert(REGISTRY_CAPACITY as u32 + 1));
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
