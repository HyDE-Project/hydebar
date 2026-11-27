//! The broadcast taps of the one compositor connection.
//!
//! Pure fan-out state: one channel per event domain and the configuration the
//! supervisor was started with. The channels are filled by [`super::wiring`]
//! and read by [`super::taps`]; the singleton in [`super::singleton`] owns
//! their lifetime.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{
    HyprlandKeyboardEvent, HyprlandWindowEvent, HyprlandWorkspaceEvent
};
use tokio::sync::broadcast;

use crate::adapters::hyprland_client::config::HyprlandClientConfig;

/// Broadcast taps of the one compositor connection.
pub struct Multiplexer {
    pub(super) window:    broadcast::Sender<HyprlandWindowEvent>,
    pub(super) workspace: broadcast::Sender<HyprlandWorkspaceEvent>,
    pub(super) keyboard:  broadcast::Sender<HyprlandKeyboardEvent>,
    pub(super) reload:    broadcast::Sender<()>,
    /// The configuration the supervisor was started with.
    ///
    /// The first caller decides the reconnect policy for everyone; a later
    /// caller with different ideas deserves to know its own were discarded.
    pub(super) config:    Arc<HyprlandClientConfig>
}
