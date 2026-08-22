//! Shared arithmetic of a sample, and the identity stamped onto it.

use sysinfo::System;

use super::{SystemInfoData, hardware, standing};

/// Stamps the machine's identity and its living readings onto a sample.
pub(super) fn stamp_environment(data: &mut SystemInfoData) {
    let identity = hardware::identity();

    data.cpu_model.clone_from(&identity.model);
    data.cpu_cores = identity.cores;
    data.cpu_max_mhz = identity.max_mhz;
    data.cpu_microcode.clone_from(&identity.microcode);
    data.kernel.clone_from(&identity.kernel);
    data.swap_backend.clone_from(&identity.swap);
    data.cpu_current_mhz = hardware::current_mhz();
    data.cpu_governor = hardware::governor();
    data.memory_cached = hardware::cached_bytes();
    data.uptime = standing::uptime();
    data.load = standing::load();
    data.tasks = standing::tasks();
    data.fans = standing::fans();
}

/// The share of every logical processor that is busy, in whole percent.
///
/// The global figure says how hard the machine is working; this says how it
/// is spread. A build using every thread and a single-threaded loop pinned to
/// one core can report the same global load and look nothing alike.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a per-core load is bounded to 0..=100"
)]
pub(super) fn per_core(system: &System) -> Vec<u32> {
    system
        .cpus()
        .iter()
        .map(|cpu| cpu.cpu_usage().clamp(0.0, 100.0) as u32)
        .collect()
}

/// Whole-percent processor load and the logical processor count.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the floored global load is bounded to 0..=100 and the logical processor count fits u32"
)]
pub(super) fn cpu_counters(system: &System) -> (u32, u32) {
    (
        system.global_cpu_usage().floor() as u32,
        system.cpus().len() as u32
    )
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "the ratio times 100 is bounded to 0..=100 and fits u32; f32 blurs only fractions below the whole percent shown"
)]
pub(super) fn percentage(used: u64, total: u64) -> u32 {
    if total == 0 {
        return 0;
    }

    ((used as f32 / total as f32) * 100.) as u32
}

/// Share and absolute amount of a memory pool in use.
///
/// `unused` is the amount a new allocation could claim, not the amount the
/// kernel currently holds free: for RAM that is `MemAvailable`, which
/// counts the reclaimable page cache as unused. Subtracting `MemFree`
/// instead would bill every cached page to the user and report a
/// machine with a warm cache as nearly full.
pub(super) fn memory_share(total: u64, unused: u64) -> (u32, u64) {
    let used = total.saturating_sub(unused);

    (percentage(used, total), used)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn percentage_handles_zero_total() {
        assert_eq!(percentage(5, 0), 0);
    }

    #[test]
    fn memory_share_reports_percentage_and_bytes() {
        assert_eq!(memory_share(1000, 250), (75, 750));
    }

    #[test]
    fn memory_share_handles_an_absent_pool() {
        assert_eq!(memory_share(0, 0), (0, 0));
    }

    #[test]
    fn memory_share_bills_only_what_cannot_be_reclaimed() {
        // 64 GiB total with 44 GiB available is 20 GiB in use, whatever the
        // page cache holds on top of the 8 GiB the kernel reports as free.
        let total = 64 * 1024 * 1024 * 1024;
        let available = 44 * 1024 * 1024 * 1024;

        assert_eq!(
            memory_share(total, available),
            (31, 20 * 1024 * 1024 * 1024)
        );
    }
}
