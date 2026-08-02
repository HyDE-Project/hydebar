//! Unit tests for the `NetworkManager` backend helpers.

use super::*;
use crate::services::network::ConnectivityState;

#[test]
fn device_type_from_u32_maps_known_values() {
    assert_eq!(DeviceType::from(2), DeviceType::Wifi);
    assert_eq!(DeviceType::from(29), DeviceType::WireGuard);
    assert_eq!(DeviceType::from(42), DeviceType::Unknown);
}

#[test]
fn connectivity_state_from_vec_prefers_highest_state() {
    let states = vec![
        ConnectivityState::Portal,
        ConnectivityState::Loss,
        ConnectivityState::Full,
    ];

    assert_eq!(ConnectivityState::from(states), ConnectivityState::Full);
}
