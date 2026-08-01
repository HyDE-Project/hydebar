//! Ending the families, politely first, and the handler for a signalled bar.

use std::{
    io, ptr,
    sync::atomic::{AtomicI32, Ordering},
    thread,
    time::Duration
};

use log::info;

use super::{
    marks::{LAUNCH_PREFIX, launch_id, terminate_marked},
    registry::LIVE_GROUPS
};

/// Time the graceful exit path is given before the process is ended anyway.
///
/// The groups are already gone by then, so this only decides how long a bar
/// whose event loop stopped answering keeps its surfaces on screen.
const HARD_EXIT_GRACE: Duration = Duration::from_millis(1000);

/// Write end of the pipe a termination signal wakes the reaper thread through.
///
/// Negative while no handler is installed.
static WAKE_WRITER: AtomicI32 = AtomicI32::new(-1);

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
pub(super) fn kill_group(pid: u32) {
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
