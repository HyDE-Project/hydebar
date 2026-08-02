//! Registration of every compositor event handler on a fresh connection.
//!
//! One connection carries every domain, so this is where each raw compositor
//! event is restated as the port event it stands for and fanned out to the
//! matching broadcast channel. Nobody listening on a channel is fine — a tap
//! nobody holds simply drops what it is sent.

use std::sync::Arc;

use hyprland::event_listener::AsyncEventListener;
use tokio::sync::broadcast;

use super::singleton::Multiplexer;
use crate::adapters::hyprland_client::HyprlandClient;

mod composite;
mod forwards;

/// Sends an event to whoever is listening; nobody listening is fine.
fn fan_out<T>(tx: &broadcast::Sender<T>, event: T) {
    let _ = tx.send(event);
}

/// Wires every handler of every domain onto one fresh connection.
pub(super) fn build_listener(
    mux: &Arc<Multiplexer>,
    client: &HyprlandClient
) -> AsyncEventListener {
    let mut listener = AsyncEventListener::new();

    forwards::register(&mut listener, mux);
    composite::register(&mut listener, mux, client);

    listener
}
