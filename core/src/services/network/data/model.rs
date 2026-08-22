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
    pub ssid:        String,
    pub strength:    u8,
    pub state:       DeviceState,
    pub public:      bool,
    pub path:        OwnedObjectPath,
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
    pub name: String,
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
    AccessPoint(AccessPoint),
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
    Wired {
        name:  String,
        speed: u32
    },
    WiFi {
        id:       String,
        name:     String,
        strength: u8
    },
    Vpn {
        name:        String,
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
    None,
    Portal,
    Loss,
    Full,
    #[default]
    Unknown
}

/// Describes the state of a device as reported by the backend.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Unmanaged,
    Unavailable,
    Disconnected,
    Prepare,
    Config,
    NeedAuth,
    IpConfig,
    IpCheck,
    Secondaries,
    Activated,
    Deactivating,
    Failed,
    #[default]
    Unknown
}
