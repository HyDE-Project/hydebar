//! Startup housekeeping: sweeping strays and arming the guards.

use log::{debug, error};

/// Clears the strays of earlier runs and arms the reaper for this one.
///
/// Ordered after the instance lock on purpose: the bar that was running has
/// already been asked to quit and released the slot, so everything still
/// wearing an older launch stamp is genuinely abandoned. Both steps are best
/// effort — a bar that cannot sweep or cannot install its handler is still a
/// working bar, and its children are covered by the parent death signal the
/// kernel enforces on each of them.
pub fn reap_and_guard_children() {
    let swept = hydebar_core::utils::process_group::sweep_orphans();

    if swept > 0 {
        debug!("ended {swept} processes left behind by an earlier run");
    }

    hydebar_core::utils::process_group::start_orphan_reaper();

    hydebar_core::outputs::start_blur_guard();

    if let Err(err) = hydebar_core::utils::process_group::claim_orphans() {
        error!("failed to claim orphaned children, some may escape the bar: {err}");
    }

    if let Err(err) = hydebar_core::utils::process_group::install_termination_handler() {
        error!("failed to arm the process reaper, a signalled exit may leave children: {err}");
    }
}
