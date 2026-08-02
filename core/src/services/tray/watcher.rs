//! Watching the session bus for tray items, with retry and rebuild on
//! failure.

use std::{future::Future, pin::Pin};

use iced::futures::Stream;
use log::error;

use super::{TrayEvent, TrayService, dbus::StatusNotifierWatcher};
use crate::services::ServiceEvent;

mod error;
mod items;
mod serve;

pub use error::TrayWatcherError;

pub type TrayEventStream = Pin<Box<dyn Stream<Item = TrayEvent> + Send + 'static>>;

/// The watcher's bus presence, owned for the life of the listener.
///
/// Dropping it aborts the bus-name watching task along with the listener,
/// so a torn-down tray module leaves neither a task nor a claimed name
/// behind — restarts replace the server instead of stacking one per retry.
struct WatcherServer {
    conn:  zbus::Connection,
    watch: tokio::task::JoinHandle<()>
}

impl Drop for WatcherServer {
    fn drop(&mut self) {
        self.watch.abort();
    }
}

pub async fn start_listening<F, Fut>(mut publisher: F)
where
    F: FnMut(ServiceEvent<TrayService>) -> Fut + Send,
    Fut: Future<Output = ()> + Send
{
    /// A serve that lasted this long counts as a healthy watch, so an
    /// isolated stumble days later starts the backoff from the bottom.
    const STABLE_WATCH: std::time::Duration = std::time::Duration::from_mins(1);

    /// Consecutive quick failures after which the connection itself is
    /// presumed dead and rebuilt from scratch.
    const REBUILD_AFTER: u32 = 3;

    loop {
        let mut failures: u32 = 0;
        let server = loop {
            match StatusNotifierWatcher::start_server().await {
                Ok((conn, watch)) => {
                    break WatcherServer {
                        conn,
                        watch
                    };
                }
                Err(err) => {
                    error!("{}", TrayWatcherError::Connection(err));
                    failures = failures.saturating_add(1);
                    tokio::time::sleep(crate::services::reconnect_delay(failures)).await;
                }
            }
        };

        let mut failures: u32 = 0;
        while failures < REBUILD_AFTER {
            let started = std::time::Instant::now();
            let err = serve::serve(&server.conn, &mut publisher).await;
            error!("{err}");

            if started.elapsed() >= STABLE_WATCH {
                failures = 0;
            }

            failures = failures.saturating_add(1);
            tokio::time::sleep(crate::services::reconnect_delay(failures)).await;
        }

        error!("the tray connection keeps failing, rebuilding it");
    }
}
