//! Rendering of the settings indicator on the bar.

use iced::{
    Element, Theme,
    widget::{Row, container}
};

use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale
    },
    menu::MenuType,
    modules::{
        OnModulePress,
        control_center::state::{ControlCenter, Message}
    }
};

impl ControlCenter {
    pub(super) fn render_bar<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        let idle_inhibited = self
            .idle_inhibitor
            .as_ref()
            .map(|i| i.is_inhibited())
            .unwrap_or(false);
        let power_profile_indicator = self
            .upower
            .as_ref()
            .and_then(|p| p.power_profile.indicator(icons));
        let sink_indicator = self.audio.as_ref().and_then(|a| a.sink_indicator(icons));
        let connection_indicator = self
            .network
            .as_ref()
            .and_then(|n| n.get_connection_indicator(icons));
        let vpn_indicator = self
            .network
            .as_ref()
            .and_then(|n| n.get_vpn_indicator(icons));
        let battery_indicator = self
            .upower
            .as_ref()
            .and_then(|upower| upower.battery)
            .map(|battery| battery.indicator(icons));

        Some((
            Row::new()
                .push_maybe(if idle_inhibited {
                    Some(
                        container(icon(icons, Icons::EyeOpened)).style(|theme: &Theme| {
                            container::Style {
                                text_color: Some(theme.palette().danger),
                                ..Default::default()
                            }
                        })
                    )
                } else {
                    None
                })
                .push_maybe(power_profile_indicator)
                .push_maybe(sink_indicator)
                .push(
                    Row::new()
                        .push_maybe(connection_indicator)
                        .push_maybe(vpn_indicator)
                        .spacing(scale::scaled(4.0))
                )
                .push_maybe(battery_indicator)
                .spacing(scale::scaled(8.0))
                .into(),
            Some(OnModulePress::ToggleMenu(MenuType::ControlCenter))
        ))
    }
}
