//! Event stream assembled from `NetworkManager` D-Bus signals.
//!
//! The list of nearby access points is deliberately absent from it. The daemon
//! republishes `AccessPoints` whenever any neighbouring radio appears or fades,
//! which on a populated band is several times a second, and answering each one
//! meant re-reading every access point over the bus and repainting every
//! surface the bar owns for a list drawn nowhere but inside the network menu.
//! The bar asks for that list on its own clock instead, and only while the user
//! is looking at the menu.

use iced::futures::{
    Stream, StreamExt,
    stream::{BoxStream, select_all}
};
use masterror::AppResult;

use super::{NetworkDbus, NetworkSettingsDbus};
use crate::services::network::NetworkEvent;

mod access_point;
mod daemon;

/// One boxed stream of network events, fallible per item.
type EventStream<'s> = BoxStream<'s, AppResult<NetworkEvent>>;

impl<'a> NetworkDbus<'a> {
    /// Assembles one stream carrying every network event the daemon signals.
    ///
    /// # Errors
    ///
    /// Returns an error when the settings service proxy cannot be created or
    /// when the devices needed for the per-device subscriptions cannot be
    /// listed.
    pub async fn subscribe_events(
        &'a self
    ) -> AppResult<impl Stream<Item = AppResult<NetworkEvent>> + 'a> {
        let conn = self.0.inner().connection();
        let settings = NetworkSettingsDbus::new(conn).await?;
        let mut streams: Vec<EventStream<'a>> = Vec::new();

        streams.push(self.wireless_enabled_events().await);
        streams.push(self.connectivity_events().await);
        streams.push(self.active_connection_events().await);
        streams.push(self.device_roster_events().await?);

        let wireless_access_points = self.wireless_access_points().await?;

        let device_state_changes = self.device_state_events(&wireless_access_points).await?;
        if !device_state_changes.is_empty() {
            let device_states = select_all(device_state_changes).boxed();
            streams.push(device_states);
        }

        let strength_changes_streams = self.strength_events(wireless_access_points).await?;
        let strength_changes = select_all(strength_changes_streams).boxed();
        streams.push(strength_changes);

        streams.push(self.known_connection_events(settings).await);

        let events = select_all(streams);

        Ok(events)
    }
}
