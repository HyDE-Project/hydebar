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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::convert::TryFrom;

    use iced_test::simulator;
    use zbus::zvariant::OwnedObjectPath;

    use super::*;
    use crate::services::network::{AccessPoint, DeviceState, Vpn};

    fn surface() -> Id {
        Id::unique()
    }

    fn icons() -> IconTheme {
        IconTheme::default()
    }

    fn path(at: &str) -> OwnedObjectPath {
        OwnedObjectPath::try_from(at).expect("a well formed object path")
    }

    fn access_point(ssid: &str, strength: u8) -> AccessPoint {
        AccessPoint {
            ssid: ssid.to_owned(),
            strength,
            state: DeviceState::Activated,
            public: true,
            path: path("/ap"),
            device_path: path("/device")
        }
    }

    fn wifi(name: &str, strength: u8) -> ActiveConnectionInfo {
        ActiveConnectionInfo::WiFi {
            id: name.to_owned(),
            name: name.to_owned(),
            strength
        }
    }

    fn vpn(name: &str) -> ActiveConnectionInfo {
        ActiveConnectionInfo::Vpn {
            name:        name.to_owned(),
            object_path: path("/vpn")
        }
    }

    fn data() -> NetworkData {
        NetworkData::default()
    }

    #[test]
    fn a_desktop_without_a_wifi_adapter_offers_no_wifi_toggle() {
        let data = data();

        assert!(
            data.get_wifi_quick_setting_button(surface(), None, false, 1.0, &icons())
                .is_none()
        );
    }

    #[test]
    fn a_desktop_with_an_adapter_offers_a_named_wifi_toggle() {
        let mut data = data();
        data.wifi_present = true;

        let (button, submenu) = data
            .get_wifi_quick_setting_button(surface(), None, false, 1.0, &icons())
            .expect("an adapter is present");

        assert!(submenu.is_none());

        let mut ui = simulator(button);
        assert!(ui.find("Wi-Fi").is_ok());
    }

    #[test]
    fn a_connected_adapter_names_the_network_and_its_strength() {
        let mut data = data();
        data.wifi_present = true;
        data.wifi_enabled = true;
        data.active_connections = vec![wifi("home", 72)];

        let (button, _) = data
            .get_wifi_quick_setting_button(surface(), None, false, 1.0, &icons())
            .expect("an adapter is present");

        let mut ui = simulator(button);
        assert!(ui.find("home (72%)").is_ok());
    }

    #[test]
    fn pressing_the_wifi_toggle_asks_to_switch_the_adapter() {
        let mut data = data();
        data.wifi_present = true;

        let (button, _) = data
            .get_wifi_quick_setting_button(surface(), None, false, 1.0, &icons())
            .expect("an adapter is present");

        let mut ui = simulator(button);
        let _ = ui.click("Wi-Fi").expect("the toggle is pressable");

        assert!(
            ui.into_messages()
                .any(|message| matches!(message, Message::Network(NetworkMessage::ToggleWiFi)))
        );
    }

    #[test]
    fn a_switched_off_adapter_offers_no_way_into_its_list() {
        let mut data = data();
        data.wifi_present = true;
        data.wifi_enabled = false;

        let (button, _) = data
            .get_wifi_quick_setting_button(surface(), Some(SubMenu::Wifi), false, 1.0, &icons())
            .expect("an adapter is present");

        let mut ui = simulator(button);
        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn the_open_wifi_submenu_lists_what_is_nearby() {
        let mut data = data();
        data.wifi_present = true;
        data.wifi_enabled = true;
        data.wireless_access_points = vec![access_point("cafe", 40)];

        let (_, submenu) = data
            .get_wifi_quick_setting_button(surface(), Some(SubMenu::Wifi), false, 1.0, &icons())
            .expect("an adapter is present");

        let mut ui = simulator(submenu.expect("the open submenu is drawn"));
        assert!(ui.find("cafe").is_ok());
    }

    #[test]
    fn another_open_submenu_does_not_open_the_wifi_one() {
        let mut data = data();
        data.wifi_present = true;
        data.wifi_enabled = true;

        let (_, submenu) = data
            .get_wifi_quick_setting_button(surface(), Some(SubMenu::Vpn), false, 1.0, &icons())
            .expect("an adapter is present");

        assert!(submenu.is_none());
    }

    #[test]
    fn a_desktop_that_remembers_no_vpn_offers_no_vpn_toggle() {
        let data = data();

        assert!(
            data.get_vpn_quick_setting_button(surface(), None, false, 1.0, &icons())
                .is_none()
        );
    }

    #[test]
    fn a_remembered_vpn_earns_a_toggle() {
        let mut data = data();
        data.known_connections = vec![KnownConnection::Vpn(Vpn {
            name: "work".to_owned(),
            path: path("/vpn")
        })];

        let (button, submenu) = data
            .get_vpn_quick_setting_button(surface(), None, false, 1.0, &icons())
            .expect("a remembered vpn is offered");

        assert!(submenu.is_none());

        let mut ui = simulator(button);
        assert!(ui.find("Vpn").is_ok());
    }

    #[test]
    fn a_remembered_access_point_alone_is_not_a_vpn() {
        let mut data = data();
        data.known_connections = vec![KnownConnection::AccessPoint(access_point("home", 60))];

        assert!(
            data.get_vpn_quick_setting_button(surface(), None, false, 1.0, &icons())
                .is_none()
        );
    }

    #[test]
    fn pressing_the_vpn_toggle_opens_its_list() {
        let mut data = data();
        data.known_connections = vec![KnownConnection::Vpn(Vpn {
            name: "work".to_owned(),
            path: path("/vpn")
        })];

        let (button, _) = data
            .get_vpn_quick_setting_button(surface(), None, false, 1.0, &icons())
            .expect("a remembered vpn is offered");

        let mut ui = simulator(button);
        let _ = ui.click("Vpn").expect("the toggle is pressable");

        assert!(
            ui.into_messages()
                .any(|message| matches!(message, Message::ToggleSubMenu(SubMenu::Vpn)))
        );
    }

    #[test]
    fn the_open_vpn_submenu_lists_what_is_remembered() {
        let mut data = data();
        data.known_connections = vec![KnownConnection::Vpn(Vpn {
            name: "work".to_owned(),
            path: path("/vpn")
        })];
        data.active_connections = vec![vpn("work")];

        let (_, submenu) = data
            .get_vpn_quick_setting_button(surface(), Some(SubMenu::Vpn), false, 1.0, &icons())
            .expect("a remembered vpn is offered");

        let mut ui = simulator(submenu.expect("the open submenu is drawn"));
        assert!(ui.find("work").is_ok());
    }

    #[test]
    fn airplane_mode_is_always_offered_and_never_opens_a_submenu() {
        let data = data();
        let (button, submenu) = data.get_airplane_mode_quick_setting_button(1.0, &icons());

        assert!(submenu.is_none());

        let mut ui = simulator(button);
        assert!(ui.find("Airplane Mode").is_ok());
    }

    #[test]
    fn pressing_airplane_mode_asks_to_switch_the_radios() {
        let data = data();
        let (button, _) = data.get_airplane_mode_quick_setting_button(1.0, &icons());

        let mut ui = simulator(button);
        let _ = ui.click("Airplane Mode").expect("the toggle is pressable");

        assert!(ui.into_messages().any(|message| matches!(
            message,
            Message::Network(NetworkMessage::ToggleAirplaneMode)
        )));
    }
}
