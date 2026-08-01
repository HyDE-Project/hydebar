use iced::{
    Alignment, Element, Length, SurfaceId as Id, Theme,
    widget::{Column, button, column, container, row, rule, scrollable, toggler}
};

use super::{Message, SubMenu, quick_setting_button};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    services::{
        ServiceEvent,
        network::{
            AccessPoint, ActiveConnectionInfo, ConnectivityState, KnownConnection,
            NetworkData, NetworkService, Vpn
        }
    },
    style::{ghost_button_style, settings_button_style},
    utils::IndicatorState
};

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

static WIFI_SIGNAL_ICONS: [Icons; 6] = [
    Icons::Wifi0,
    Icons::Wifi1,
    Icons::Wifi2,
    Icons::Wifi3,
    Icons::Wifi4,
    Icons::Wifi5
];

static WIFI_LOCK_SIGNAL_ICONS: [Icons; 5] = [
    Icons::WifiLock1,
    Icons::WifiLock2,
    Icons::WifiLock3,
    Icons::WifiLock4,
    Icons::WifiLock5
];

impl ActiveConnectionInfo {
    /// Maps a signal strength to its icon bucket, whatever the backend
    /// sends.
    ///
    /// The strength is clamped first: a backend can report a value past one
    /// hundred — a wrapped negative RSSI does exactly that — and an index
    /// computed from it unclamped walked off the end of the icon tables.
    fn signal_bucket(signal: u8) -> usize {
        f32::round(f32::from(signal.min(100)) / 100. * 4.) as usize
    }

    pub fn get_wifi_icon(signal: u8) -> Icons {
        WIFI_SIGNAL_ICONS[1 + Self::signal_bucket(signal)]
    }

    pub fn get_wifi_lock_icon(signal: u8) -> Icons {
        WIFI_LOCK_SIGNAL_ICONS[Self::signal_bucket(signal)]
    }

    pub fn get_icon(&self) -> Icons {
        match self {
            Self::WiFi {
                strength, ..
            } => Self::get_wifi_icon(*strength),
            Self::Wired {
                ..
            } => Icons::Ethernet,
            Self::Vpn {
                ..
            } => Icons::Vpn
        }
    }

    pub fn get_indicator_state(&self) -> IndicatorState {
        match self {
            Self::WiFi {
                strength: 0 | 1, ..
            } => IndicatorState::Warning,
            _ => IndicatorState::Normal
        }
    }
}

impl super::ControlCenter {
    /// One-look summary of the connection, for the pointer resting on the
    /// network module.
    pub fn network_hint(&self) -> Option<String> {
        self.network
            .as_ref()
            .map(|service| service.connection_hint())
    }
}

impl NetworkData {
    /// States the connection the way its hover reads: the network, the
    /// signal, the frequency, the interface, the addressing — every fact
    /// the bar holds, one per line — or the one word explaining why
    /// there is nothing to state.
    pub fn connection_hint(&self) -> String {
        let mut lines = Vec::new();
        let mut vpns = Vec::new();

        for connection in &self.active_connections {
            match connection {
                ActiveConnectionInfo::WiFi {
                    name,
                    strength,
                    ..
                } => {
                    lines.push(format!("Network: {name}"));
                    lines.push(match self.link.signal_dbm {
                        Some(dbm) => format!("Signal strength: {dbm}dBm ({strength}%)"),
                        None => format!("Signal strength: {strength}%")
                    });

                    if let Some(mhz) = self.link.frequency_mhz {
                        lines.push(format!("Frequency: {mhz}MHz"));
                    }
                }
                ActiveConnectionInfo::Wired {
                    name,
                    speed
                } => {
                    lines.push(format!("Wired: {name}"));

                    if *speed > 0 {
                        lines.push(format!("Speed: {speed} Mb/s"));
                    }
                }
                ActiveConnectionInfo::Vpn {
                    name, ..
                } => vpns.push(name.clone())
            }
        }

        if !lines.is_empty() {
            if let Some(interface) = &self.link.interface {
                lines.push(format!("Interface: {interface}"));
            }

            if let Some(address) = &self.link.address {
                lines.push(format!("IP: {address}"));
            }

            if let Some(gateway) = &self.link.gateway {
                lines.push(format!("Gateway: {gateway}"));
            }

            if let Some(netmask) = &self.link.netmask {
                lines.push(format!("Netmask: {netmask}"));
            }
        }

        for vpn in vpns {
            lines.push(format!("VPN: {vpn}"));
        }

        if lines.is_empty() {
            lines.push(
                if self.airplane_mode {
                    "Airplane mode"
                } else if self.wifi_present && !self.wifi_enabled {
                    "Wi-Fi off"
                } else {
                    "Disconnected"
                }
                .to_owned()
            );
        }

        lines.join("\n")
    }

    pub fn get_connection_indicator<Message: 'static>(
        &self,
        icons: &IconTheme
    ) -> Option<Element<'static, Message>> {
        if self.airplane_mode || !self.wifi_present {
            None
        } else {
            Some(
                self.active_connections
                    .iter()
                    .find(|c| {
                        matches!(c, ActiveConnectionInfo::WiFi { .. })
                            || matches!(c, ActiveConnectionInfo::Wired { .. })
                    })
                    .map_or_else(
                        || icon(icons, Icons::Wifi0).into(),
                        |a| {
                            let icon_type = a.get_icon();
                            let state = (self.connectivity, a.get_indicator_state());

                            container(icon(icons, icon_type))
                                .style(move |theme: &Theme| container::Style {
                                    text_color: match state {
                                        (ConnectivityState::Full, IndicatorState::Warning) => {
                                            Some(theme.palette().warning)
                                        }
                                        (ConnectivityState::Full, _) => None,
                                        _ => Some(theme.palette().danger)
                                    },
                                    ..Default::default()
                                })
                                .into()
                        }
                    )
            )
        }
    }

    pub fn get_vpn_indicator<Message: 'static>(
        &self,
        icons: &IconTheme
    ) -> Option<Element<'static, Message>> {
        self.active_connections
            .iter()
            .find(|c| matches!(c, ActiveConnectionInfo::Vpn { .. }))
            .map(|a| {
                let icon_type = a.get_icon();

                container(icon(icons, icon_type))
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.extended_palette().danger.weak.color),
                        ..Default::default()
                    })
                    .into()
            })
    }

    pub fn get_wifi_quick_setting_button(
        &self,
        id: Id,
        sub_menu: Option<SubMenu>,
        show_more_button: bool,
        opacity: f32,
        icons: &IconTheme
    ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
        if self.wifi_present {
            let active_connection = self.active_connections.iter().find_map(|c| match c {
                ActiveConnectionInfo::WiFi {
                    name,
                    strength,
                    ..
                } => Some((name, strength, c.get_icon())),
                _ => None
            });

            Some((
                quick_setting_button(
                    icons,
                    active_connection.map_or_else(|| Icons::Wifi0, |(_, _, icon)| icon),
                    "Wi-Fi".to_string(),
                    active_connection
                        .map(|(name, strength, _)| format!("{name} ({}%)", strength,)),
                    self.wifi_enabled,
                    Message::Network(NetworkMessage::ToggleWiFi),
                    self.wifi_enabled.then(|| {
                        (
                            SubMenu::Wifi,
                            sub_menu,
                            Message::ToggleSubMenu(SubMenu::Wifi)
                        )
                    }),
                    opacity
                ),
                sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Wifi)
                    .map(|_| {
                        self.wifi_menu(
                            id,
                            active_connection
                                .map(|(name, strengh, _)| (name.as_str(), *strengh)),
                            show_more_button,
                            opacity,
                            icons
                        )
                        .map(Message::Network)
                    })
            ))
        } else {
            None
        }
    }

    pub fn get_vpn_quick_setting_button(
        &self,
        id: Id,
        sub_menu: Option<SubMenu>,
        show_more_button: bool,
        opacity: f32,
        icons: &IconTheme
    ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
        self.known_connections
            .iter()
            .any(|c| matches!(c, KnownConnection::Vpn { .. }))
            .then(|| {
                (
                    quick_setting_button(
                        icons,
                        Icons::Vpn,
                        "Vpn".to_string(),
                        None,
                        self.active_connections
                            .iter()
                            .any(|c| matches!(c, ActiveConnectionInfo::Vpn { .. })),
                        Message::ToggleSubMenu(SubMenu::Vpn),
                        None,
                        opacity
                    ),
                    sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Vpn)
                        .map(|_| {
                            self.vpn_menu(id, show_more_button, opacity)
                                .map(Message::Network)
                        })
                )
            })
    }

    pub fn wifi_menu(
        &self,
        id: Id,
        active_connection: Option<(&str, u8)>,
        show_more_button: bool,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, NetworkMessage> {
        let main = column!(
        row!(
            text("Nearby Wifi").width(Length::Fill),
            text(if self.scanning_nearby_wifi {
                "Scanning..."
            } else {
                ""
            })
            .size(scale::scaled(12.0)),
            button(icon(icons, Icons::Refresh))
                .padding([scale::scaled(4.0), scale::scaled(10.0)])
                .style(settings_button_style(opacity))
                .on_press(NetworkMessage::ScanNearByWiFi),
        )
        .spacing(scale::scaled(8.0))
        .width(Length::Fill)
        .align_y(Alignment::Center),
        rule::horizontal(1),
        container(scrollable(
            Column::with_children(
                self.wireless_access_points
                .iter()
                .filter_map(|ac| if active_connection.is_some_and(|(ssid, _)| ssid == ac.ssid) {Some((ac, true))} else {None })
                .chain(self.wireless_access_points
                    .iter()
                    .filter_map(|ac| if active_connection.is_some_and(|(ssid, _)| ssid == ac.ssid) {None} else {Some((ac, false))})
                )
                    .map(|(ac, is_active)| {
                        let is_known = self.known_connections.iter().any(|c| {
                            matches!(
                                c,
                                KnownConnection::AccessPoint(AccessPoint { ssid, .. }) if ssid == &ac.ssid
                            )
                        });

                        let row_button = button(
                            container(
                                row!(
                                    icon(icons, if ac.public {
                                        ActiveConnectionInfo::get_wifi_icon(ac.strength)
                                    } else {
                                        ActiveConnectionInfo::get_wifi_lock_icon(ac.strength)
                                    })
                                    .width(Length::Shrink),
                                    text(ac.ssid.clone()).width(Length::Fill),
                                    text(format!("{}%", ac.strength)).size(scale::scaled(12.0)),
                                )
                                .align_y(Alignment::Center)
                                .spacing(scale::scaled(8.0)),
                            )
                            .style(move |theme: &Theme| {
                                container::Style {
                                    text_color: if is_active {
                                        Some(theme.palette().success)
                                    } else {
                                        None
                                    },
                                    ..Default::default()
                                }
                            }),
                        )
                        .style(ghost_button_style(opacity))
                        .padding([scale::scaled(8.0), scale::scaled(8.0)])
                        .width(Length::Fill);

                    let press = (!is_active).then(|| {
                        if is_known {
                            NetworkMessage::SelectAccessPoint(ac.clone())
                        } else {
                            NetworkMessage::RequestWiFiPassword(id, ac.ssid.clone())
                        }
                    });

                    match press {
                        Some(message) => iced::widget::mouse_area(row_button)
                            .on_press(message)
                            .into(),
                        None => row_button.into()
                    }
                    })
                    .collect::<Vec<Element<NetworkMessage>>>(),
            )
            .spacing(scale::scaled(4.0))
        ))
        .max_height(200),
    )
    .width(Length::Fill)
    .spacing(scale::scaled(8.0));

        if show_more_button {
            column!(
                main,
                rule::horizontal(1),
                button("More")
                    .on_press(NetworkMessage::WiFiMore(id))
                    .padding([scale::scaled(4.0), scale::scaled(12.0)])
                    .width(Length::Fill)
                    .style(ghost_button_style(opacity))
            )
            .spacing(scale::scaled(12.0))
            .into()
        } else {
            main.into()
        }
    }

    pub fn vpn_menu(
        &self,
        id: Id,
        show_more_button: bool,
        opacity: f32
    ) -> Element<'_, NetworkMessage> {
        let main = Column::with_children(
        self.known_connections
            .iter()
            .filter_map(|c| match c {
                KnownConnection::Vpn(vpn) => Some(vpn),
                _ => None,
            })
            .map(|vpn| {
                let is_active = self.active_connections.iter().any(
                    |c| matches!(c, ActiveConnectionInfo::Vpn { name, .. } if name == &vpn.name),
                );

                row!(
                    text(vpn.name.to_string()).width(Length::Fill),
                    toggler(is_active)
                        .on_toggle(|_| { NetworkMessage::ToggleVpn(vpn.clone()) })
                        .width(Length::Shrink),
                )
                .into()
            })
            .collect::<Vec<Element<NetworkMessage>>>(),
    )
    .width(Length::Fill)
    .spacing(scale::scaled(8.0));

        if show_more_button {
            column!(
                main,
                rule::horizontal(1),
                button("More")
                    .on_press(NetworkMessage::VpnMore(id))
                    .padding([scale::scaled(4.0), scale::scaled(12.0)])
                    .width(Length::Fill)
                    .style(ghost_button_style(opacity))
            )
            .spacing(scale::scaled(12.0))
            .into()
        } else {
            main.into()
        }
    }

    pub fn get_airplane_mode_quick_setting_button(
        &self,
        opacity: f32,
        icons: &IconTheme
    ) -> (Element<'_, Message>, Option<Element<'_, Message>>) {
        (
            quick_setting_button(
                icons,
                Icons::Airplane,
                "Airplane Mode".to_string(),
                None,
                self.airplane_mode,
                Message::Network(NetworkMessage::ToggleAirplaneMode),
                None,
                opacity
            ),
            None
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_possible_signal_yields_a_wifi_icon_without_panicking() {
        for signal in u8::MIN..=u8::MAX {
            let _ = ActiveConnectionInfo::get_wifi_icon(signal);
        }
    }

    #[test]
    fn every_possible_signal_yields_a_wifi_lock_icon_without_panicking() {
        for signal in u8::MIN..=u8::MAX {
            let _ = ActiveConnectionInfo::get_wifi_lock_icon(signal);
        }
    }

    #[test]
    fn signal_quartiles_pick_ascending_wifi_icons() {
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(0), Icons::Wifi1);
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(25), Icons::Wifi2);
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(50), Icons::Wifi3);
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(75), Icons::Wifi4);
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(100), Icons::Wifi5);
    }

    #[test]
    fn signal_quartiles_pick_ascending_wifi_lock_icons() {
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(0),
            Icons::WifiLock1
        );
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(25),
            Icons::WifiLock2
        );
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(50),
            Icons::WifiLock3
        );
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(75),
            Icons::WifiLock4
        );
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(100),
            Icons::WifiLock5
        );
    }

    #[test]
    fn a_signal_past_one_hundred_stays_in_the_top_bucket() {
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(u8::MAX), Icons::Wifi5);
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(u8::MAX),
            Icons::WifiLock5
        );
    }

    #[test]
    fn the_hover_states_the_wifi_and_its_strength() {
        let data = NetworkData {
            active_connections: vec![ActiveConnectionInfo::WiFi {
                id:       "home".to_owned(),
                name:     "HomeNet".to_owned(),
                strength: 87
            }],
            ..NetworkData::default()
        };

        assert_eq!(
            data.connection_hint(),
            "Network: HomeNet\nSignal strength: 87%"
        );
    }

    #[test]
    fn the_hover_states_every_fact_of_the_link_it_holds() {
        let data = NetworkData {
            active_connections: vec![ActiveConnectionInfo::WiFi {
                id:       "home".to_owned(),
                name:     "HomeNet".to_owned(),
                strength: 87
            }],
            link: crate::services::network::LinkDetails {
                interface:     Some("wlan0".to_owned()),
                signal_dbm:    Some(-27),
                frequency_mhz: Some(5320),
                address:       Some("192.168.2.19/24".to_owned()),
                gateway:       Some("192.168.2.253".to_owned()),
                netmask:       Some("255.255.255.0".to_owned())
            },
            ..NetworkData::default()
        };

        assert_eq!(
            data.connection_hint(),
            "Network: HomeNet\nSignal strength: -27dBm (87%)\nFrequency: \
         5320MHz\nInterface: wlan0\nIP: 192.168.2.19/24\nGateway: \
         192.168.2.253\nNetmask: 255.255.255.0"
        );
    }

    #[test]
    fn the_hover_states_the_wire_and_any_vpn_on_top() {
        let data = NetworkData {
            active_connections: vec![
                ActiveConnectionInfo::Wired {
                    name:  "eth0".to_owned(),
                    speed: 1000
                },
                ActiveConnectionInfo::Vpn {
                    name:        "work".to_owned(),
                    object_path: zbus::zvariant::OwnedObjectPath::try_from("/").expect("path")
                },
            ],
            ..NetworkData::default()
        };

        assert_eq!(
            data.connection_hint(),
            "Wired: eth0\nSpeed: 1000 Mb/s\nVPN: work"
        );
    }

    #[test]
    fn a_wire_without_a_reported_speed_keeps_that_line_to_itself() {
        let data = NetworkData {
            active_connections: vec![ActiveConnectionInfo::Wired {
                name:  "eth0".to_owned(),
                speed: 0
            }],
            ..NetworkData::default()
        };

        assert_eq!(data.connection_hint(), "Wired: eth0");
    }

    #[test]
    fn nothing_connected_names_the_reason() {
        assert_eq!(NetworkData::default().connection_hint(), "Disconnected");

        let airplane = NetworkData {
            airplane_mode: true,
            ..NetworkData::default()
        };
        assert_eq!(airplane.connection_hint(), "Airplane mode");

        let radio_off = NetworkData {
            wifi_present: true,
            wifi_enabled: false,
            ..NetworkData::default()
        };
        assert_eq!(radio_off.connection_hint(), "Wi-Fi off");
    }
}
