//! Tuning of the process-wide system allocator.
//!
//! glibc malloc gives every thread that allocates concurrently its own arena,
//! up to eight per core, and each arena keeps holding on to the memory the
//! threads freed into it. A long-lived bar frees icon buffers, sample tables
//! and view trees all day, and the freed pages pile up in arenas no thread
//! visits again — the resident set creeps upward and never comes back down.
//!
//! The tuning is the one the Mandara shell measured to win against mimalloc
//! and jemalloc on a live Wayland shell: at most [`ARENA_MAX`] arenas, every
//! allocation from [`MMAP_THRESHOLD`] up given its own mapping (returned to
//! the OS the moment it is freed), and a [`malloc_trim`] whenever the heap's
//! free bytes have grown by [`TRIM_GROWTH`] since the last check — so an
//! idle bar never walks its heap at all.
//!
//! On non-GNU targets (musl, whose mallocng hands freed memory back by
//! itself and exposes none of these knobs) every entry point here is a no-op.

use std::time::Duration;

/// glibc arenas the allocator may create at most.
#[cfg(target_env = "gnu")]
const ARENA_MAX: libc::c_int = 2;

/// Allocations from this size up get their own mapping, returned to the OS
/// on free.
///
/// Setting it also disables glibc's dynamic threshold, which climbs after
/// the first large free and keeps large buffers — decoded pictures, sample
/// tables — inside an arena where longer-lived allocations would pin them.
#[cfg(target_env = "gnu")]
const MMAP_THRESHOLD: libc::c_int = 512 * 1024;

/// How often the upkeep task checks whether freed heap is worth handing back.
const TRIM_EVERY: Duration = Duration::from_secs(30);

/// Growth of the heap's free bytes between two checks that earns a trim.
const TRIM_GROWTH: usize = 1024 * 1024;

/// Whether the free bytes of the heap rose enough since `before` to trim.
///
/// What a trim leaves behind is the next check's baseline, so a heap that
/// stays fragmented but quiet is not trimmed again and again.
#[must_use]
pub const fn trim_due(before: usize, now: usize) -> bool {
    now.saturating_sub(before) >= TRIM_GROWTH
}

/// Applies the allocator tuning that must happen before any other thread
/// exists, called first thing in `main`.
pub fn init() {
    #[cfg(target_env = "gnu")]
    #[expect(
        unsafe_code,
        reason = "mallopt only changes glibc malloc parameters; called before any other thread exists"
    )]
    unsafe {
        libc::mallopt(libc::M_ARENA_MAX, ARENA_MAX);
        libc::mallopt(libc::M_MMAP_THRESHOLD, MMAP_THRESHOLD);
    }
}

/// Starts the periodic heap upkeep on `handle`: every [`TRIM_EVERY`], trim
/// the heap only if its free bytes grew by [`TRIM_GROWTH`] since the check
/// before. A no-op where the system allocator has no trim to give.
pub fn start_upkeep(handle: &tokio::runtime::Handle) {
    #[cfg(target_env = "gnu")]
    handle.spawn(async {
        let mut seen = free_heap_bytes();

        loop {
            tokio::time::sleep(TRIM_EVERY).await;
            let now = free_heap_bytes();

            if trim_due(seen, now) {
                trim();
            }

            seen = now;
        }
    });
}

/// Bytes malloc holds free in its arenas (mmapped chunks are returned on
/// free and are not counted).
#[cfg(target_env = "gnu")]
fn free_heap_bytes() -> usize {
    #[expect(
        unsafe_code,
        reason = "mallinfo2 only reads allocator statistics and is thread-safe"
    )]
    let info = unsafe { libc::mallinfo2() };

    info.fordblks
}

/// Hands the heap's free pages back to the OS.
#[cfg(target_env = "gnu")]
fn trim() {
    #[expect(
        unsafe_code,
        reason = "malloc_trim is thread-safe and has no caller preconditions"
    )]
    unsafe {
        libc::malloc_trim(0);
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn only_growth_of_the_free_heap_earns_a_trim() {
        assert!(!trim_due(10 << 20, 10 << 20));
        assert!(!trim_due(10 << 20, (10 << 20) + TRIM_GROWTH - 1));
        assert!(trim_due(10 << 20, (10 << 20) + TRIM_GROWTH));
    }

    #[test]
    fn a_shrinking_heap_never_earns_a_trim() {
        assert!(!trim_due(10 << 20, 1 << 20));
        assert!(!trim_due(usize::MAX, 0));
    }
}
