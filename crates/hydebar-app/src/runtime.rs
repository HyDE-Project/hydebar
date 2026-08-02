//! Sizing and construction of the async runtime backing the bar.

use tokio::runtime::Runtime;

use crate::error::MainError;

/// Worker threads backing every asynchronous source the bar owns.
///
/// The default pool is sized to the CPU count, which on a desktop machine means
/// dozens of workers for a process whose entire workload is parking on D-Bus,
/// Wayland, Hyprland and child-process pipes. A fixed handful covers the only
/// tasks that ever hold a worker for longer than a poll — the synchronous
/// Hyprland round-trips issued from the listener handlers and the `sysinfo`
/// sampler reading `/proc` — while leaving spare capacity so none of them can
/// starve the others.
const RUNTIME_WORKER_THREADS: usize = 4;

/// Builds the runtime whose handle the bar drives its sources with.
pub fn build() -> Result<Runtime, MainError> {
    let workers = std::env::var("HYDEBAR_RUNTIME_THREADS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(RUNTIME_WORKER_THREADS);

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .thread_name("hydebar-rt")
        .enable_all()
        .build()
        .map_err(MainError::Runtime)
}
