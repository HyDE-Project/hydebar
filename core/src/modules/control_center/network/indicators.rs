//! Bar indicators of the connection and of any VPN riding on it.

use iced::{Element, Theme, widget::container};

use crate::{
    components::icons::{IconTheme, Icons, icon},
    services::network::{ActiveConnectionInfo, ConnectivityState, NetworkData},
    utils::IndicatorState
};

impl NetworkData {
    #[must_use]
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

    #[must_use]
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
}
