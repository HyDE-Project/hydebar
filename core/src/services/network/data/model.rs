//! Snapshot types describing the network state a backend reports.

use zbus::zvariant::OwnedObjectPath;

use super::NetworkServiceError;

/// Facts about the live link, read beside the backend rather than from it.
///
/// # Examples
/// ```
/// use hydebar_core::services::network::LinkDetails;
///
/// let details = LinkDetails::default();
/// assert!(details.interface.is_none());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LinkDetails {
    /// Interface the default route rides on.
    pub interface:     Option<String>,
    /// Wireless signal, in dBm.
    pub signal_dbm:    Option<i32>,
    /// Wireless channel frequency, in MHz.
    pub frequency_mhz: Option<u32>,
    /// First IPv4 address with its prefix, `addr/len`.
    pub address:       Option<String>,
    /// Gateway of the default route.
    pub gateway:       Option<String>,
    /// Netmask of that address, spelled dotted.
    pub netmask:       Option<String>
}

/// Collection of data maintained by the [`NetworkService`].
///
/// # Examples
/// ```
/// use hydebar_core::services::network::{ConnectivityState, NetworkData};
///
/// let data = NetworkData::default();
/// assert!(matches!(data.connectivity, ConnectivityState::Unknown));
/// ```
#[derive(Debug, Default, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag mirrors an independent daemon property; packing them into enums would change the public data model"
)]
pub struct NetworkData {
    /// Whether a Wi-Fi adapter is present.
    pub wifi_present:           bool,
    /// Discovered wireless access points.
    pub wireless_access_points: Vec<AccessPoint>,
    /// Active network connections reported by the backend.
    pub active_connections:     Vec<ActiveConnectionInfo>,
    /// Connections remembered by the backend.
    pub known_connections:      Vec<KnownConnection>,
    /// Whether Wi-Fi is enabled.
    pub wifi_enabled:           bool,
    /// Whether airplane mode is active.
    pub airplane_mode:          bool,
    /// Connectivity status reported by the backend.
    pub connectivity:           ConnectivityState,
    /// Whether the backend is scanning for Wi-Fi.
    pub scanning_nearby_wifi:   bool,
    /// Facts about the link the default route rides on.
    pub link:                   LinkDetails,
    /// The last error encountered by the service, if any.
    pub last_error:             Option<NetworkServiceError>
}

/// Describes a Wi-Fi access point.
///
/// # Examples
/// ```
/// use std::convert::TryFrom;
///
/// use hydebar_core::services::network::{AccessPoint, DeviceState};
/// use zbus::zvariant::OwnedObjectPath;
///
/// let ap = AccessPoint {
///     ssid:        "example".into(),
///     strength:    42,
///     state:       DeviceState::Activated,
///     public:      true,
///     path:        OwnedObjectPath::try_from("/").unwrap(),
///     device_path: OwnedObjectPath::try_from("/").unwrap()
/// };
/// assert_eq!(ap.ssid, "example");
/// ```
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AccessPoint {
    /// Name the network broadcasts itself under.
    pub ssid:        String,
    /// Signal, as a share of full scale.
    pub strength:    u8,
    /// State of the device that can reach it.
    pub state:       DeviceState,
    /// Whether the network is open, needing no secret.
    pub public:      bool,
    /// Where the backend keeps the network on the bus.
    pub path:        OwnedObjectPath,
    /// Where the backend keeps the device that reaches it.
    pub device_path: OwnedObjectPath
}

/// Describes a VPN entry.
///
/// # Examples
/// ```
/// use std::convert::TryFrom;
///
/// use hydebar_core::services::network::Vpn;
/// use zbus::zvariant::OwnedObjectPath;
///
/// let vpn = Vpn {
///     name: "work".into(),
///     path: OwnedObjectPath::try_from("/").unwrap()
/// };
/// assert_eq!(vpn.name, "work");
/// ```
#[derive(Debug, Clone)]
pub struct Vpn {
    /// Name the connection is configured under.
    pub name: String,
    /// Where the backend keeps the connection on the bus.
    pub path: OwnedObjectPath
}

/// Known connections stored by the backend.
///
/// # Examples
/// ```
/// use std::convert::TryFrom;
///
/// use hydebar_core::services::network::{AccessPoint, DeviceState, KnownConnection};
/// use zbus::zvariant::OwnedObjectPath;
///
/// let ap = AccessPoint {
///     ssid:        "lab".into(),
///     strength:    0,
///     state:       DeviceState::Unknown,
///     public:      true,
///     path:        OwnedObjectPath::try_from("/").unwrap(),
///     device_path: OwnedObjectPath::try_from("/").unwrap()
/// };
/// let connection = KnownConnection::AccessPoint(ap);
/// assert!(matches!(connection, KnownConnection::AccessPoint(_)));
/// ```
#[derive(Debug, Clone)]
pub enum KnownConnection {
    /// A wireless network the machine has settings for.
    AccessPoint(AccessPoint),
    /// A VPN the machine has settings for.
    Vpn(Vpn)
}

/// Active connection information summarised by the backend.
///
/// # Examples
/// ```
/// use std::convert::TryFrom;
///
/// use hydebar_core::services::network::ActiveConnectionInfo;
/// use zbus::zvariant::OwnedObjectPath;
///
/// let info = ActiveConnectionInfo::Vpn {
///     name:        "vpn".into(),
///     object_path: OwnedObjectPath::try_from("/").unwrap()
/// };
/// assert_eq!(info.name(), "vpn");
/// ```
#[derive(Debug, Clone)]
pub enum ActiveConnectionInfo {
    /// A link over a cable.
    Wired {
        /// Name of the connection.
        name:  String,
        /// Negotiated speed, in megabits per second.
        speed: u32
    },
    /// A link over the air.
    WiFi {
        /// Identifier the backend addresses the connection by.
        id:       String,
        /// Name the network broadcasts itself under.
        name:     String,
        /// Signal, as a share of full scale.
        strength: u8
    },
    /// A tunnel over whichever link is carrying it.
    Vpn {
        /// Name the connection is configured under.
        name:        String,
        /// Where the backend keeps the connection on the bus.
        object_path: OwnedObjectPath
    }
}

impl ActiveConnectionInfo {
    /// Returns the human-friendly name of the connection.
    ///
    /// # Examples
    /// ```
    /// use hydebar_core::services::network::ActiveConnectionInfo;
    /// use zbus::zvariant::OwnedObjectPath;
    ///
    /// let info = ActiveConnectionInfo::Vpn {
    ///     name:        "vpn".into(),
    ///     object_path: OwnedObjectPath::try_from("/").unwrap()
    /// };
    /// assert_eq!(info.name(), "vpn");
    /// ```
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::Wired {
                name, ..
            }
            | Self::WiFi {
                name, ..
            }
            | Self::Vpn {
                name, ..
            } => name.clone()
        }
    }
}

/// Describes the system connectivity status.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityState {
    /// Nothing is reachable.
    None,
    /// A captive portal stands between the machine and the network.
    Portal,
    /// The link is up and the wider network is not answering.
    Loss,
    /// Everything is reachable.
    Full,
    /// The backend has not said, or said something new.
    #[default]
    Unknown
}

/// Describes the state of a device as reported by the backend.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// The backend is not driving this device.
    Unmanaged,
    /// The device is there and cannot be used.
    Unavailable,
    /// The device is idle and holds no connection.
    Disconnected,
    /// The device is getting ready to connect.
    Prepare,
    /// The device is being configured for a connection.
    Config,
    /// The connection is waiting for a secret.
    NeedAuth,
    /// The device is asking for an address.
    IpConfig,
    /// The address the device was given is being checked.
    IpCheck,
    /// The connection is waiting on a dependent one.
    Secondaries,
    /// The connection is up and carrying traffic.
    Activated,
    /// The connection is being taken down.
    Deactivating,
    /// The connection failed.
    Failed,
    /// The backend has not said, or said something new.
    #[default]
    Unknown
}
