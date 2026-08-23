//! Network face of the control center: messages, icons, indicators,
//! quick settings and submenus.

use iced::SurfaceId as Id;

use crate::services::{
    ServiceEvent,
    network::{AccessPoint, NetworkService, Vpn}
};

mod hint;
mod icons;
mod indicators;
mod quick_settings;
mod vpn_menu;
mod wifi_menu;

/// What the network section of the quick settings answers to.
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    /// The backend said something.
    Event(Box<ServiceEvent<NetworkService>>),
    /// Turn the wireless radio on or off.
    ToggleWiFi,
    /// Look for networks in reach again.
    ScanNearByWiFi,
    /// Open the full network settings.
    WiFiMore(Id),
    /// Open the full VPN settings.
    VpnMore(Id),
    /// Join this network.
    SelectAccessPoint(AccessPoint),
    /// Ask for the secret this network needs.
    RequestWiFiPassword(Id, String),
    /// Bring this tunnel up, or take it down.
    ToggleVpn(Vpn),
    /// Turn every radio off, or back on.
    ToggleAirplaneMode
}
