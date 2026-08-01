//! Termination requests coming from outside the process.
//!
//! A newly started bar asks the running one to quit with `SIGTERM` before it
//! takes over the single instance lock, and session managers do the same on
//! logout. Both have to reach the event loop as a message: the surfaces are
//! owned by the runtime, so only the update loop can take them off the screen
//! before the process exits.

use std::{any::TypeId, hash::Hash, thread, time::Duration};

use iced::{
    Subscription,
    futures::stream::{BoxStream, StreamExt, once}
};
use iced_futures::subscription::{self, Recipe, from_recipe};
use log::{error, info};
use tokio::signal::unix::{SignalKind, signal};

/// Time the runtime is given to confirm the surface removal before the
/// process ends regardless.
///
/// The confirmation is a message chained behind the destroy tasks; the
/// backstop only fires when that message never lands — a stalled runtime or
/// a compositor that stopped answering.
const FLUSH_BACKSTOP: Duration = Duration::from_secs(2);

/// Ends the process now, with the module commands ended first.
///
/// Stopping the iced runtime instead would tear the graphics context down, a
/// teardown that both outlives the takeover window of the successor and is
/// prone to crashing once the layer surfaces are gone. Exiting the process
/// releases the single instance lock at once.
///
/// `std::process::exit` runs no destructor, so the listener shells are ended
/// explicitly: they would otherwise survive the bar that started them, and
/// every takeover would add another set of loops spawning helpers on a timer.
pub(super) fn exit_now() -> ! {
    hydebar_core::utils::process_group::terminate_all();
    info!("surfaces removed, exiting");
    std::process::exit(0);
}

/// Ends the process even if the surface-removal confirmation never arrives.
pub(super) fn exit_backstop() {
    thread::spawn(|| {
        thread::sleep(FLUSH_BACKSTOP);
        error!("surface removal was never confirmed, exiting anyway");
        hydebar_core::utils::process_group::terminate_all();
        std::process::exit(0);
    });
}

/// Signal that asked the bar to quit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownSignal {
    /// A takeover or a session manager asked the process to terminate.
    Terminate,
    /// The terminal the bar was started from was interrupted.
    Interrupt
}

/// Subscription resolving once the process is asked to quit.
pub(super) fn subscription() -> Subscription<ShutdownSignal> {
    from_recipe(ShutdownWatcher)
}

struct ShutdownWatcher;

impl Recipe for ShutdownWatcher {
    type Output = ShutdownSignal;

    fn hash(&self, state: &mut subscription::Hasher) {
        TypeId::of::<Self>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream
    ) -> BoxStream<'static, Self::Output> {
        once(wait_for_signal())
            .filter_map(|signal| async move { signal })
            .boxed()
    }
}

/// Waits for the first termination signal, reporting which one arrived.
async fn wait_for_signal() -> Option<ShutdownSignal> {
    let mut terminate = match signal(SignalKind::terminate()) {
        Ok(stream) => stream,
        Err(err) => {
            error!("failed to listen for SIGTERM, takeover will be abrupt: {err}");

            return None;
        }
    };
    let mut interrupt = match signal(SignalKind::interrupt()) {
        Ok(stream) => stream,
        Err(err) => {
            error!("failed to listen for SIGINT: {err}");

            return None;
        }
    };

    let received = tokio::select! {
        _ = terminate.recv() => ShutdownSignal::Terminate,
        _ = interrupt.recv() => ShutdownSignal::Interrupt
    };
    info!("shutdown requested: {received:?}");

    Some(received)
}
