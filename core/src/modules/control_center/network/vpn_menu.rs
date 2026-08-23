//! The unfolded VPN submenu: every known tunnel with its toggle.

use iced::{
    Element, Length, SurfaceId as Id,
    widget::{Column, button, column, row, rule, toggler}
};

use super::NetworkMessage;
use crate::{
    components::{scale, text::text},
    services::network::{ActiveConnectionInfo, KnownConnection, NetworkData},
    style::ghost_button_style
};

impl NetworkData {
    /// The list of tunnels the machine has settings for.
    #[must_use]
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
                    KnownConnection::AccessPoint(_) => None
                })
                .map(|vpn| {
                    let is_active = self.active_connections.iter().any(
                    |c| matches!(c, ActiveConnectionInfo::Vpn { name, .. } if name == &vpn.name),
                );

                    row!(
                        text(vpn.name.clone()).width(Length::Fill),
                        toggler(is_active)
                            .on_toggle(|_| { NetworkMessage::ToggleVpn(vpn.clone()) })
                            .width(Length::Shrink),
                    )
                    .into()
                })
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
}
