//! Reading a station's connection state the way the rest of the bar states it.
//!
//! iwd reports the state of a wireless station as one of a handful of words on
//! the `net.connman.iwd.Station` interface, while every consumer of the network
//! service reads [`DeviceState`]. The words are translated here so a station
//! that iwd calls "connecting" is drawn by the same rule that draws a
//! `NetworkManager` device halfway through association.

use crate::services::network::DeviceState;

/// Translates the word iwd uses for a station into the shared device state.
///
/// A word the daemon adds later reads as [`DeviceState::Unknown`], which is
/// what the bar already draws for a device it cannot describe.
#[must_use]
pub(super) fn state_from_station(state: &str) -> DeviceState {
    match state {
        "connected" => DeviceState::Activated,
        "connecting" | "roaming" => DeviceState::Config,
        "disconnecting" => DeviceState::Deactivating,
        "disconnected" => DeviceState::Disconnected,
        _ => DeviceState::Unknown
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{DeviceState, state_from_station};

    #[test]
    fn a_connected_station_reads_as_an_activated_device() {
        assert_eq!(state_from_station("connected"), DeviceState::Activated);
    }

    #[test]
    fn a_station_halfway_there_reads_as_a_device_being_configured() {
        assert_eq!(state_from_station("connecting"), DeviceState::Config);
        assert_eq!(state_from_station("roaming"), DeviceState::Config);
    }

    #[test]
    fn a_station_on_its_way_out_reads_as_a_deactivating_device() {
        assert_eq!(
            state_from_station("disconnecting"),
            DeviceState::Deactivating
        );
    }

    #[test]
    fn a_station_holding_no_connection_reads_as_a_disconnected_device() {
        assert_eq!(
            state_from_station("disconnected"),
            DeviceState::Disconnected
        );
    }

    #[test]
    fn a_word_the_daemon_adds_later_reads_as_an_undescribed_device() {
        assert_eq!(state_from_station(""), DeviceState::Unknown);
        assert_eq!(state_from_station("scanning"), DeviceState::Unknown);
    }
}
