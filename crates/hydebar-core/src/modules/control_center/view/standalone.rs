//! Bar entries and menus of the modules split out of the control center.
//!
//! Audio, network, bluetooth and the power profile each render their own bar
//! entry and own menu, while the services behind them stay in the single
//! [`ControlCenter`] state: splitting the presentation must not multiply the
//! D-Bus connections the bar keeps open.

use iced::{
    Element, Length, SurfaceId as Id,
    widget::{Column, Row}
};

use super::{
    helpers::{quick_settings_section, sub_menu_wrapper},
    quick_setting_button
};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale
    },
    config::{ControlCenterModuleConfig, Position},
    menu::MenuType,
    modules::{
        OnModulePress,
        control_center::{
            power::power_menu,
            state::{ControlCenter, Message, SubMenu}
        }
    },
    password_dialog,
    services::bluetooth::BluetoothState
};

impl ControlCenter {
    /// Bar entry of the standalone audio module.
    ///
    /// Renders nothing while the audio service is away, so a session without a
    /// sound server keeps a bar free of dead icons.
    pub fn audio_bar<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        let indicator = self.audio.as_ref().and_then(|a| a.sink_indicator(icons))?;

        Some((indicator, Some(OnModulePress::ToggleMenu(MenuType::Audio))))
    }

    /// Bar entry of the standalone network module, connection and VPN.
    pub fn network_bar<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        let network = self.network.as_ref()?;
        let connection = network.get_connection_indicator(icons);
        let vpn = network.get_vpn_indicator(icons);

        if connection.is_none() && vpn.is_none() {
            return None;
        }

        Some((
            Row::new()
                .push_maybe(connection)
                .push_maybe(vpn)
                .spacing(scale::scaled(4.0))
                .into(),
            Some(OnModulePress::ToggleMenu(MenuType::Network))
        ))
    }

    /// Bar entry of the standalone bluetooth module.
    ///
    /// A machine without a bluetooth radio reports the state as unavailable and
    /// the module stays off the bar.
    pub fn bluetooth_bar<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        self.bluetooth
            .as_ref()
            .filter(|b| b.state != BluetoothState::Unavailable)?;

        Some((
            icon(icons, Icons::Bluetooth).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Bluetooth))
        ))
    }

    /// Bar entry of the standalone power profile module.
    pub fn power_profile_bar<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        let indicator = self
            .upower
            .as_ref()
            .and_then(|p| p.power_profile.indicator(icons))?;

        Some((
            indicator,
            Some(OnModulePress::ToggleMenu(MenuType::PowerProfile))
        ))
    }

    /// Menu of the standalone audio module: both sliders and their device
    /// lists.
    pub fn audio_menu(
        &self,
        id: Id,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        position: Position,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let (sink_slider, source_slider) = self
            .audio
            .as_ref()
            .map(|a| a.audio_sliders(self.sub_menu, opacity, icons))
            .unwrap_or((None, None));

        let (top_sink_slider, bottom_sink_slider) = match position {
            Position::Top => (sink_slider, None),
            Position::Bottom => (None, sink_slider)
        };
        let (top_source_slider, bottom_source_slider) = match position {
            Position::Top => (source_slider, None),
            Position::Bottom => (None, source_slider)
        };

        Column::new()
            .push_maybe(top_sink_slider)
            .push_maybe(
                self.sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Sinks)
                    .and_then(|_| {
                        self.audio.as_ref().map(|a| {
                            sub_menu_wrapper(
                                a.sinks_submenu(
                                    id,
                                    config.audio_sinks_more_cmd.is_some(),
                                    opacity,
                                    icons
                                ),
                                opacity
                            )
                        })
                    })
            )
            .push_maybe(bottom_sink_slider)
            .push_maybe(top_source_slider)
            .push_maybe(
                self.sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Sources)
                    .and_then(|_| {
                        self.audio.as_ref().map(|a| {
                            sub_menu_wrapper(
                                a.sources_submenu(
                                    id,
                                    config.audio_sources_more_cmd.is_some(),
                                    opacity,
                                    icons
                                ),
                                opacity
                            )
                        })
                    })
            )
            .push_maybe(bottom_source_slider)
            .width(Length::Fill)
            .spacing(scale::scaled(16.0))
            .into()
    }

    /// Menu of the standalone network module: connection, VPN and airplane
    /// mode.
    ///
    /// The password prompt of a protected network belongs here as well, since
    /// this is the menu the connection attempt starts from.
    pub fn network_menu(
        &self,
        id: Id,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        if let Some((ssid, current_password)) = &self.password_dialog {
            return password_dialog::view(id, ssid, current_password, opacity, icons)
                .map(Message::PasswordDialog);
        }

        let buttons = vec![
            self.network.as_ref().and_then(|n| {
                n.get_wifi_quick_setting_button(
                    id,
                    self.sub_menu,
                    config.wifi_more_cmd.is_some(),
                    opacity,
                    icons
                )
            }),
            self.network.as_ref().and_then(|n| {
                n.get_vpn_quick_setting_button(
                    id,
                    self.sub_menu,
                    config.vpn_more_cmd.is_some(),
                    opacity,
                    icons
                )
            }),
            self.network.as_ref().and_then(|n| {
                if config.remove_airplane_btn {
                    None
                } else {
                    Some(n.get_airplane_mode_quick_setting_button(opacity, icons))
                }
            }),
        ];

        quick_settings_section(buttons.into_iter().flatten().collect::<Vec<_>>(), opacity)
    }

    /// Menu of the standalone bluetooth module.
    pub fn bluetooth_menu(
        &self,
        id: Id,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let button = self
            .bluetooth
            .as_ref()
            .filter(|b| b.state != BluetoothState::Unavailable)
            .and_then(|b| {
                b.get_quick_setting_button(
                    id,
                    self.sub_menu,
                    config.bluetooth_more_cmd.is_some(),
                    opacity,
                    icons
                )
            });

        quick_settings_section(button.into_iter().collect::<Vec<_>>(), opacity)
    }

    /// Menu of the standalone power profile module, with the power actions
    /// underneath.
    pub fn power_profile_menu(
        &self,
        opacity: f32,
        config: &ControlCenterModuleConfig,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let profile = self
            .upower
            .as_ref()
            .and_then(|u| u.power_profile.get_quick_setting_button(opacity, icons));

        Column::new()
            .push(quick_settings_section(
                profile.into_iter().collect::<Vec<_>>(),
                opacity
            ))
            .push(power_menu(opacity, config, icons).map(Message::Power))
            .width(Length::Fill)
            .spacing(scale::scaled(16.0))
            .into()
    }

    /// Quick toggle of the idle inhibitor, shared by the control center menu.
    #[allow(dead_code)]
    pub(super) fn idle_quick_button(
        &self,
        opacity: f32,
        icons: &IconTheme
    ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
        self.idle_inhibitor.as_ref().map(|inhibitor| {
            (
                quick_setting_button(
                    icons,
                    if inhibitor.is_inhibited() {
                        Icons::EyeOpened
                    } else {
                        Icons::EyeClosed
                    },
                    "Idle Inhibitor".to_string(),
                    None,
                    inhibitor.is_inhibited(),
                    Message::ToggleInhibitIdle,
                    None,
                    opacity
                ),
                None
            )
        })
    }
}
