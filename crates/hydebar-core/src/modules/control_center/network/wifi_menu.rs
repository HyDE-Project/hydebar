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
