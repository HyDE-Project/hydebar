//! Quick setting toggles of the network family: Wi-Fi, VPN and
//! airplane mode.

use iced::{Element, SurfaceId as Id};

use super::{
    super::{Message, SubMenu, quick_setting_button},
    NetworkMessage
};
use crate::{
    components::icons::{IconTheme, Icons},
    services::network::{ActiveConnectionInfo, KnownConnection, NetworkData}
};

impl NetworkData {
    #[must_use]
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
                    active_connection.map(|(name, strength, _)| format!("{name} ({strength}%)")),
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
                            active_connection.map(|(name, strengh, _)| (name.as_str(), *strengh)),
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

    #[must_use]
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

    #[must_use]
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
