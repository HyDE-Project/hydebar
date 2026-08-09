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

/// Ceiling on the threads the blocking pool may hold at once.
///
/// Every synchronous errand the bar runs — the compositor round trips behind a
/// snapshot, the `sysinfo` sampler, a theme's swatches, a directory of
/// wallpapers, a tray icon being decoded — is handed to this pool, and the
/// default ceiling is five hundred and twelve. Startup is the busiest the
/// pool ever gets and it takes eight; a ceiling at twice the observed peak
/// leaves that untouched while denying any burst the right to answer with
/// hundreds of threads.
const BLOCKING_THREAD_CEILING: usize = 16;

/// Workers left to a runtime the bar does not build itself.
///
/// The widget toolkit builds one of its own for subscriptions, sized to the
/// CPU count because nothing tells it otherwise — thirty-two worker threads on
/// a thirty-two-thread desktop, for work that is a handful of streams. The
/// count is not settable through the toolkit, so it is stated the one way the
/// runtime reads without being asked.
const BORROWED_WORKER_THREADS: &str = "2";

/// Variable a runtime reads its default worker count from.
const WORKER_COUNT_KEY: &str = "TOKIO_WORKER_THREADS";

/// Caps the pool of every runtime built without the bar's say-so.
///
/// Must be called before the process grows a second thread: the value is
/// stated in the environment, which is shared, and reading it while another
/// thread writes it is what makes the call unsound rather than merely
/// untidy. An operator who set the variable themselves is left alone.
pub fn cap_borrowed_pools() {
    if std::env::var_os(WORKER_COUNT_KEY).is_some() {
        return;
    }

    #[expect(
        unsafe_code,
        reason = "called from the first line of startup, while the process is still single-threaded"
    )]
    unsafe {
        std::env::set_var(WORKER_COUNT_KEY, BORROWED_WORKER_THREADS);
    }
}

/// Builds the runtime whose handle the bar drives its sources with.
pub fn build() -> Result<Runtime, MainError> {
    let workers = std::env::var("HYDEBAR_RUNTIME_THREADS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(RUNTIME_WORKER_THREADS);

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .max_blocking_threads(BLOCKING_THREAD_CEILING)
        .thread_name("hydebar-rt")
        .enable_all()
        .build()
        .map_err(MainError::Runtime)
}
