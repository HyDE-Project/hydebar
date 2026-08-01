//! The stamps that tell the bar's processes apart from everyone else's.

use std::{
    fs,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering}
    },
    time::{SystemTime, UNIX_EPOCH}
};

use log::warn;

/// Environment variable stamping every process the bar supervises.
///
/// Its value identifies the run that started the process, which is what lets a
/// later bar tell its own children from the strays of a previous one.
pub const LAUNCH_VAR: &str = "HYDEBAR_LAUNCH_ID";

/// [`LAUNCH_VAR`] as it appears in a `/proc/<pid>/environ` entry.
pub(super) const LAUNCH_PREFIX: &[u8] = b"HYDEBAR_LAUNCH_ID=";

/// Environment variable stamping one supervised spawn.
///
/// The launch stamp tells this bar's processes from an earlier bar's, which is
/// too coarse to cancel a single command: ending everything carrying it would
/// take every other listener down as well. This one narrows the same trick to
/// one spawn, so a cancelled command can be followed to the descendants that
/// left its process group behind.
pub const SPAWN_VAR: &str = "HYDEBAR_SPAWN_ID";

/// [`SPAWN_VAR`] as it appears in a `/proc/<pid>/environ` entry.
pub(super) const SPAWN_PREFIX: &[u8] = b"HYDEBAR_SPAWN_ID=";

/// Serial number handed to the next supervised spawn.
static SPAWN_COUNTER: AtomicU64 = AtomicU64::new(0);

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
pub(super) fn next_spawn_id() -> String {
    let serial = SPAWN_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{}-{serial}", launch_id())
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
pub(super) fn terminate_detached(spawn: &str) -> usize {
    terminate_marked(SPAWN_PREFIX, |stamp| stamp == spawn)
}

/// Ends every process whose stamp under `prefix` is accepted by `wanted`.
///
/// Two passes, because the first one may land between a shell loop starting a
/// helper and the scan noticing it; the helper carries the same stamp through
/// the inherited environment, so the second pass finds it.
pub(super) fn terminate_marked(prefix: &[u8], wanted: impl Fn(&str) -> bool) -> usize {
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
pub(super) fn marked_processes(prefix: &[u8], wanted: &impl Fn(&str) -> bool) -> Vec<u32> {
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
pub(super) fn marked_stamp<'a>(environ: &'a [u8], prefix: &[u8]) -> Option<&'a str> {
    environ.split(|byte| *byte == 0).find_map(|entry| {
        let stamp = entry.strip_prefix(prefix)?;

        std::str::from_utf8(stamp).ok()
    })
}
