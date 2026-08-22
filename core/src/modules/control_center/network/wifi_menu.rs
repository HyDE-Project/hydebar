//! The unfolded Wi-Fi submenu: the list of nearby networks.

use iced::{
    Alignment, Element, Length, SurfaceId as Id, Theme,
    widget::{Column, button, column, container, row, rule, scrollable}
};

use super::NetworkMessage;
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    services::network::{AccessPoint, ActiveConnectionInfo, KnownConnection, NetworkData},
    style::{ghost_button_style, settings_button_style}
};

impl NetworkData {
    #[must_use]
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
                    }),
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
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::convert::TryFrom;

    use iced_test::simulator;
    use zbus::zvariant::OwnedObjectPath;

    use super::*;
    use crate::services::network::DeviceState;

    fn surface() -> Id {
        Id::unique()
    }

    fn icons() -> IconTheme {
        IconTheme::default()
    }

    fn path(at: &str) -> OwnedObjectPath {
        OwnedObjectPath::try_from(at).expect("a well formed object path")
    }

    fn access_point(ssid: &str, strength: u8, public: bool) -> AccessPoint {
        AccessPoint {
            ssid: ssid.to_owned(),
            strength,
            state: DeviceState::Activated,
            public,
            path: path("/ap"),
            device_path: path("/device")
        }
    }

    fn data(points: Vec<AccessPoint>, known: Vec<KnownConnection>) -> NetworkData {
        NetworkData {
            wifi_present: true,
            wifi_enabled: true,
            wireless_access_points: points,
            known_connections: known,
            ..NetworkData::default()
        }
    }

    #[test]
    fn the_menu_heads_the_list_and_offers_a_rescan() {
        let data = data(vec![], vec![]);

        let mut ui = simulator(data.wifi_menu(surface(), None, false, 1.0, &icons()));

        assert!(ui.find("Nearby Wifi").is_ok());
        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn a_scan_in_flight_says_so() {
        let mut data = data(vec![], vec![]);
        data.scanning_nearby_wifi = true;

        let mut ui = simulator(data.wifi_menu(surface(), None, false, 1.0, &icons()));

        assert!(ui.find("Scanning...").is_ok());
    }

    #[test]
    fn a_settled_adapter_says_nothing_about_scanning() {
        let data = data(vec![], vec![]);

        let mut ui = simulator(data.wifi_menu(surface(), None, false, 1.0, &icons()));

        assert!(ui.find("Scanning...").is_err());
    }

    #[test]
    fn every_nearby_network_is_named_with_its_strength() {
        let data = data(
            vec![
                access_point("cafe", 44, true),
                access_point("home", 88, false),
            ],
            vec![]
        );

        let mut ui = simulator(data.wifi_menu(surface(), None, false, 1.0, &icons()));

        assert!(ui.find("cafe").is_ok());
        assert!(ui.find("44%").is_ok());
        assert!(ui.find("home").is_ok());
        assert!(ui.find("88%").is_ok());
    }

    #[test]
    fn the_network_in_use_is_listed_first() {
        let data = data(
            vec![
                access_point("cafe", 44, true),
                access_point("home", 88, true),
            ],
            vec![]
        );

        let mut ui =
            simulator(data.wifi_menu(surface(), Some(("home", 88)), false, 1.0, &icons()));

        let joined = ui
            .find("home")
            .expect("the joined network is listed")
            .visible_bounds()
            .expect("the joined network is visible");
        let other = ui
            .find("cafe")
            .expect("the other network is listed")
            .visible_bounds()
            .expect("the other network is visible");

        assert!(joined.y < other.y);
    }

    #[test]
    fn the_network_in_use_answers_no_press() {
        let data = data(vec![access_point("home", 88, true)], vec![]);

        let mut ui =
            simulator(data.wifi_menu(surface(), Some(("home", 88)), false, 1.0, &icons()));
        let _ = ui.click("home").expect("the joined network is listed");

        assert!(
            ui.into_messages().next().is_none(),
            "joining the network already joined is not a deed"
        );
    }

    #[test]
    fn a_remembered_network_is_joined_without_asking_for_a_password() {
        let point = access_point("home", 88, false);
        let data = data(
            vec![point.clone()],
            vec![KnownConnection::AccessPoint(point)]
        );

        let mut ui = simulator(data.wifi_menu(surface(), None, false, 1.0, &icons()));
        let _ = ui.click("home").expect("the network is listed");

        assert!(ui.into_messages().any(
            |message| matches!(message, NetworkMessage::SelectAccessPoint(chosen)
                    if chosen.ssid == "home")
        ));
    }

    #[test]
    fn a_network_nobody_remembers_asks_for_its_password() {
        let data = data(vec![access_point("cafe", 44, false)], vec![]);

        let mut ui = simulator(data.wifi_menu(surface(), None, false, 1.0, &icons()));
        let _ = ui.click("cafe").expect("the network is listed");

        assert!(ui.into_messages().any(
            |message| matches!(message, NetworkMessage::RequestWiFiPassword(_, ssid)
                    if ssid == "cafe")
        ));
    }

    #[test]
    fn an_open_and_a_locked_network_are_drawn_apart() {
        let data = data(
            vec![
                access_point("open", 60, true),
                access_point("locked", 60, false),
            ],
            vec![]
        );

        let mut ui = simulator(data.wifi_menu(surface(), None, false, 1.0, &icons()));

        assert!(ui.find("open").is_ok());
        assert!(ui.find("locked").is_ok());
        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn pressing_refresh_asks_for_a_scan() {
        let data = data(vec![], vec![]);

        let mut ui = simulator(data.wifi_menu(surface(), None, false, 1.0, &icons()));
        ui.point_at(iced::Point::new(1010.0, 12.0));
        let _ = ui.simulate(iced_test::simulator::click());

        assert!(
            ui.into_messages()
                .any(|message| matches!(message, NetworkMessage::ScanNearByWiFi))
        );
    }

    #[test]
    fn a_menu_that_can_show_more_carries_the_offer() {
        let data = data(vec![], vec![]);

        let mut ui = simulator(data.wifi_menu(surface(), None, true, 1.0, &icons()));

        assert!(ui.find("More").is_ok());
    }

    #[test]
    fn a_menu_that_holds_everything_offers_nothing_more() {
        let data = data(vec![], vec![]);

        let mut ui = simulator(data.wifi_menu(surface(), None, false, 1.0, &icons()));

        assert!(ui.find("More").is_err());
    }

    #[test]
    fn pressing_more_asks_the_list_to_open_wider() {
        let data = data(vec![], vec![]);

        let mut ui = simulator(data.wifi_menu(surface(), None, true, 1.0, &icons()));
        let _ = ui.click("More").expect("the offer is pressable");

        assert!(
            ui.into_messages()
                .any(|message| matches!(message, NetworkMessage::WiFiMore(_)))
        );
    }
}
