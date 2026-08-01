//! Rendering of the settings menu contents.

use iced::{
    Element, Length, SurfaceId as Id,
    widget::{Column, Row, Space, button}
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
    modules::control_center::{
        power::power_menu,
        state::{ControlCenter, Message, SubMenu}
    },
    password_dialog,
    services::bluetooth::BluetoothState,
    style::settings_button_style
};

impl ControlCenter {
    /// Builds the top row of the menu: the battery readout and the lock and
    /// power buttons.
    fn menu_header(
        &self,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let battery_data = self
            .upower
            .as_ref()
            .and_then(|upower| upower.battery)
            .map(|battery| battery.settings_indicator(icons));
        let right_buttons = Row::new()
            .push_maybe(config.lock_cmd.as_ref().map(|_| {
                button(icon(icons, Icons::Lock))
                    .padding([scale::scaled(8.0), scale::scaled(13.0)])
                    .on_press(Message::Lock)
                    .style(settings_button_style(opacity))
            }))
            .push(
                button(icon(
                    icons,
                    if self.sub_menu == Some(SubMenu::Power) {
                        Icons::Close
                    } else {
                        Icons::Power
                    }
                ))
                .padding([scale::scaled(8.0), scale::scaled(13.0)])
                .on_press(Message::ToggleSubMenu(SubMenu::Power))
                .style(settings_button_style(opacity))
            )
            .spacing(scale::scaled(8.0));

        Row::new()
            .push_maybe(battery_data)
            .push(Space::new().width(Length::Fill))
            .push(right_buttons)
            .spacing(scale::scaled(8.0))
            .width(Length::Fill)
            .into()
    }

    /// Builds the grid of quick setting toggles and their unfolded menus.
    fn menu_quick_settings(
        &self,
        id: Id,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let wifi_setting_button = self.network.as_ref().and_then(|n| {
            n.get_wifi_quick_setting_button(
                id,
                self.sub_menu,
                config.wifi_more_cmd.is_some(),
                opacity,
                icons
            )
        });

        quick_settings_section(
            vec![
                wifi_setting_button,
                self.bluetooth
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
                self.idle_inhibitor.as_ref().and_then(|i| {
                    if config.remove_idle_btn {
                        None
                    } else {
                        Some((
                            quick_setting_button(
                                icons,
                                if i.is_inhibited() {
                                    Icons::EyeOpened
                                } else {
                                    Icons::EyeClosed
                                },
                                "Idle Inhibitor".to_string(),
                                None,
                                i.is_inhibited(),
                                Message::ToggleInhibitIdle,
                                None,
                                opacity
                            ),
                            None
                        ))
                    }
                }),
                self.upower
                    .as_ref()
                    .and_then(|u| u.power_profile.get_quick_setting_button(opacity, icons)),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
            opacity
        )
    }

    pub(super) fn render_menu(
        &self,
        id: Id,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        position: Position,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        if let Some((ssid, current_password)) = &self.password_dialog {
            password_dialog::view(id, ssid, current_password, opacity, icons)
                .map(Message::PasswordDialog)
        } else {
            let header = self.menu_header(config, opacity, icons);

            let (sink_slider, source_slider) = self.audio.as_ref().map_or((None, None), |a| {
                a.audio_sliders(self.sub_menu, opacity, icons)
            });

            let quick_settings = self.menu_quick_settings(id, config, opacity, icons);

            let (top_sink_slider, bottom_sink_slider) = match position {
                Position::Top => (sink_slider, None),
                Position::Bottom => (None, sink_slider)
            };
            let (top_source_slider, bottom_source_slider) = match position {
                Position::Top => (source_slider, None),
                Position::Bottom => (None, source_slider)
            };

            Column::new()
                .push(header)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Power)
                        .map(|_| {
                            sub_menu_wrapper(
                                power_menu(opacity, config, icons).map(Message::Power),
                                opacity
                            )
                        })
                )
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
                .push_maybe(self.brightness.as_ref().map(|b| b.brightness_slider(icons)))
                .push(quick_settings)
                .width(Length::Fill)
                .spacing(scale::scaled(16.0))
                .into()
        }
    }
}
