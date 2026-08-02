//! The subscription surface every consumer of compositor events taps.
//!
//! Each function hands out a broadcast tap of the shared connection, wrapped
//! in the stream shape the port promises. When the multiplexer cannot start,
//! the tap is a stream that ends immediately instead of hanging — every
//! subscriber treats a closed stream as a failure and retries, which retries
//! the multiplexer too.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{
    HyprlandEventStream, HyprlandKeyboardEvent, HyprlandWindowEvent, HyprlandWorkspaceEvent
};
use log::{error, warn};
use tokio::sync::broadcast;

use super::singleton::multiplexer;
use crate::adapters::hyprland_client::{HyprlandClient, config::HyprlandClientConfig};

/// The stream handed out when the multiplexer cannot start.
///
/// It ends immediately instead of hanging: every subscriber treats a closed
/// stream as a failure and retries, which retries the multiplexer too.
fn dead_stream<T: Send + 'static>(
    err: &hydebar_proto::ports::hyprland::HyprlandError
) -> HyprlandEventStream<T> {
    error!(target: "hydebar::hyprland", "compositor events unavailable: {err}");

    Box::pin(iced::futures::stream::empty())
}

/// Subscribes to the window events of the shared connection.
pub fn window_events(
    client: &HyprlandClient,
    config: &Arc<HyprlandClientConfig>
) -> HyprlandEventStream<HyprlandWindowEvent> {
    match multiplexer(client, config) {
        Ok(mux) => stream_from(mux.window.subscribe()),
        Err(err) => dead_stream(&err)
    }
}

/// Subscribes to the workspace events of the shared connection.
pub fn workspace_events(
    client: &HyprlandClient,
    config: &Arc<HyprlandClientConfig>
) -> HyprlandEventStream<HyprlandWorkspaceEvent> {
    match multiplexer(client, config) {
        Ok(mux) => stream_from(mux.workspace.subscribe()),
        Err(err) => dead_stream(&err)
    }
}

/// Subscribes to the keyboard events of the shared connection.
pub fn keyboard_events(
    client: &HyprlandClient,
    config: &Arc<HyprlandClientConfig>
) -> HyprlandEventStream<HyprlandKeyboardEvent> {
    match multiplexer(client, config) {
        Ok(mux) => stream_from(mux.keyboard.subscribe()),
        Err(err) => dead_stream(&err)
    }
}

/// Subscribes to the compositor's configuration reloads.
///
/// The receiver supports blocking reads, so the blur guard can keep its plain
/// thread while sharing the one connection with everything else.
pub fn config_reloads(
    client: &HyprlandClient,
    config: &Arc<HyprlandClientConfig>
) -> broadcast::Receiver<()> {
    match multiplexer(client, config) {
        Ok(mux) => mux.reload.subscribe(),
        Err(err) => {
            error!(target: "hydebar::hyprland", "compositor reloads unavailable: {err}");

            broadcast::channel(1).0.subscribe()
        }
    }
}

/// Wraps a broadcast tap into the stream shape the port promises.
///
/// A subscriber that fell behind is told how much it missed and keeps
/// reading: every consumer of these events re-reads the compositor state
/// anyway, so a gap costs one refresh, not correctness.
fn stream_from<T: Clone + Send + 'static>(rx: broadcast::Receiver<T>) -> HyprlandEventStream<T> {
    Box::pin(iced::futures::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(event) => return Some((Ok(event), rx)),
                Err(broadcast::error::RecvError::Lagged(missed)) => {
                    warn!(
                        target: "hydebar::hyprland",
                        "event subscriber lagged, {missed} events were skipped"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => return None
            }
        }
    }))
}
