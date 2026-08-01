//! Runtime the compositor event sockets are read on.
//!
//! Every other source the bar owns — D-Bus services, timers, child processes —
//! shares one runtime with the user interface. Handing the compositor sockets
//! to that runtime as well makes a workspace switch wait behind whatever the
//! shared runtime happens to be doing, and an idle runtime parks its reactor:
//! the switch is then read only once some unrelated wakeup happens to poll the
//! socket again, which is the difference between a bar that follows the
//! compositor and one that looks asleep.
//!
//! The listeners therefore own a small runtime whose entire workload is the
//! compositor sockets. Its reactor has nothing else to wait on, so a line
//! written by the compositor is read on the next scheduler wakeup, and the
//! events it publishes reach the shared runtime by waking a task rather than by
//! waiting for the shared reactor to come back around.

use std::{sync::OnceLock, thread};

use hydebar_proto::ports::hyprland::HyprlandError;
use tokio::runtime::{Builder, Handle};

/// Threads the listener runtime schedules its tasks on.
///
/// One thread reads the sockets. The second keeps a listener that has to ask
/// the compositor a question before it can publish — the keyboard listener does
/// on a configuration reload — from holding up the sockets of the others.
const LISTENER_WORKER_THREADS: usize = 2;

/// Name the listener runtime gives its worker threads.
const LISTENER_THREAD_NAME: &str = "hydebar-hypr";

/// Handle of the listener runtime, built the first time a listener starts.
static LISTENER_RUNTIME: OnceLock<Handle> = OnceLock::new();

/// Builds the listener runtime and leaks it, returning its handle.
///
/// The runtime lives as long as the process: the listeners are torn down by
/// dropping their streams, never by shutting the runtime down, and a runtime
/// dropped while its tasks still run would abort them mid-reconnect.
fn build() -> Result<Handle, HyprlandError> {
    let runtime = Builder::new_multi_thread()
        .worker_threads(LISTENER_WORKER_THREADS)
        .thread_name(LISTENER_THREAD_NAME)
        .enable_all()
        .build()
        .map_err(|err| {
            HyprlandError::message(
                "listener_runtime",
                format!("failed to build the compositor listener runtime: {err}")
            )
        })?;
    let handle = runtime.handle().clone();

    thread::Builder::new()
        .name(LISTENER_THREAD_NAME.to_owned())
        .spawn(move || {
            let _runtime = runtime;
            thread::park();
        })
        .map_err(|err| {
            HyprlandError::message(
                "listener_runtime",
                format!("failed to start the compositor listener thread: {err}")
            )
        })?;

    Ok(handle)
}

/// Handle of the runtime the compositor listeners run on.
///
/// # Errors
///
/// Returns [`HyprlandError::Message`] when the runtime backing the listeners
/// could not be started.
pub fn handle() -> Result<&'static Handle, HyprlandError> {
    if let Some(handle) = LISTENER_RUNTIME.get() {
        return Ok(handle);
    }

    let handle = build()?;

    Ok(LISTENER_RUNTIME.get_or_init(|| handle))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::handle;

    #[test]
    fn the_runtime_is_built_once() {
        let first = handle().expect("the listener runtime starts");
        let second = handle().expect("the listener runtime is reused");

        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn a_timer_on_the_listener_runtime_fires() {
        let handle = handle().expect("the listener runtime starts");
        let started = Instant::now();

        handle
            .block_on(async {
                tokio::time::timeout(
                    Duration::from_secs(5),
                    tokio::time::sleep(Duration::from_millis(20))
                )
                .await
            })
            .expect("the listener runtime drives its own timers");

        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
