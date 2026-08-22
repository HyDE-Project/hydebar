//! Events emitted by the network service and commands it accepts.

use super::{
    AccessPoint, ActiveConnectionInfo, ConnectivityState, KnownConnection, LinkDetails, Vpn
};

/// Describes network-related events emitted by the [`NetworkService`].
///
/// # Examples
/// ```
/// use hydebar_core::services::network::NetworkEvent;
/// let event = NetworkEvent::ScanningNearbyWifi;
/// assert!(matches!(event, NetworkEvent::ScanningNearbyWifi));
/// ```
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// Indicates that Wi-Fi has been enabled or disabled.
    WiFiEnabled(bool),
    /// Indicates that airplane mode has been enabled or disabled.
    AirplaneMode(bool),
    /// Provides the current connectivity state.
    Connectivity(ConnectivityState),
    /// Carries information about wireless devices and access points.
    WirelessDevice {
        /// Whether a Wi-Fi adapter is present on the system.
        wifi_present:           bool,
        /// Visible access points for the adapter.
        wireless_access_points: Vec<AccessPoint>
    },
    /// Lists currently active connections.
    ActiveConnections(Vec<ActiveConnectionInfo>),
    /// Lists connections remembered by the backend.
    KnownConnections(Vec<KnownConnection>),
    /// Provides an updated snapshot of visible access points.
    WirelessAccessPoint(Vec<AccessPoint>),
    /// Contains a signal strength update for an SSID.
    Strength((String, u8)),
    /// Requests a password for the given SSID.
    RequestPasswordForSSID(String),
    /// Indicates that the backend is scanning for Wi-Fi networks.
    ScanningNearbyWifi,
    /// Carries fresh facts about the link the default route rides on.
    LinkDetails(LinkDetails)
}

/// Commands accepted by the [`NetworkService`].
///
/// # Examples
/// ```
/// use std::convert::TryFrom;
///
/// use hydebar_core::services::network::{AccessPoint, DeviceState, NetworkCommand};
/// use zbus::zvariant::OwnedObjectPath;
///
/// let command = NetworkCommand::ScanNearByWiFi;
/// assert!(matches!(command, NetworkCommand::ScanNearByWiFi));
///
/// let ap = AccessPoint {
///     ssid:        "test".into(),
///     strength:    0,
///     state:       DeviceState::Unknown,
///     public:      true,
///     path:        OwnedObjectPath::try_from("/").unwrap(),
///     device_path: OwnedObjectPath::try_from("/").unwrap()
/// };
/// let _ = NetworkCommand::SelectAccessPoint((ap, None));
/// ```
#[derive(Debug, Clone)]
pub enum NetworkCommand {
    /// Request a Wi-Fi scan.
    ScanNearByWiFi,
    /// Toggle Wi-Fi enablement.
    ToggleWiFi,
    /// Toggle airplane mode.
    ToggleAirplaneMode,
    /// Request connection to an access point.
    SelectAccessPoint((AccessPoint, Option<String>)),
    /// Toggle a VPN connection.
    ToggleVpn(Vpn)
}
