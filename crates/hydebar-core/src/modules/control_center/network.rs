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

#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Event(Box<ServiceEvent<NetworkService>>),
    ToggleWiFi,
    ScanNearByWiFi,
    WiFiMore(Id),
    VpnMore(Id),
    SelectAccessPoint(AccessPoint),
    RequestWiFiPassword(Id, String),
    ToggleVpn(Vpn),
    ToggleAirplaneMode
}
