//! Rendering of the settings menu contents.

use iced::{
    Element, Length,
    widget::{Column, Row, Space, button},
    window::Id
};

use super::{
    helpers::{quick_settings_section, sub_menu_wrapper},
    quick_setting_button
};
use crate::{
    components::icons::{Icons, icon},
    config::{Position, SettingsModuleConfig},
    modules::settings::{
        power::power_menu,
        state::{Message, Settings, SubMenu}
    },
    password_dialog,
    services::bluetooth::BluetoothState,
    style::settings_button_style
};

impl Settings {
    pub(super) fn render_menu(
        &self,
        id: Id,
        config: &SettingsModuleConfig,
        opacity: f32,
        position: Position
    ) -> Element<'_, Message> {
        if let Some((ssid, current_password)) = &self.password_dialog {
            password_dialog::view(id, ssid, current_password, opacity).map(Message::PasswordDialog)
        } else {
            let battery_data = self
                .upower
                .as_ref()
                .and_then(|upower| upower.battery)
                .map(|battery| battery.settings_indicator());
            let right_buttons = Row::new()
                .push_maybe(config.lock_cmd.as_ref().map(|_| {
                    button(icon(Icons::Lock))
                        .padding([8, 13])
                        .on_press(Message::Lock)
                        .style(settings_button_style(opacity))
                }))
                .push(
                    button(icon(if self.sub_menu == Some(SubMenu::Power) {
                        Icons::Close
                    } else {
                        Icons::Power
                    }))
                    .padding([8, 13])
                    .on_press(Message::ToggleSubMenu(SubMenu::Power))
                    .style(settings_button_style(opacity))
                )
                .spacing(8);

            let header = Row::new()
                .push_maybe(battery_data)
                .push(Space::new().width(Length::Fill))
                .push(right_buttons)
                .spacing(8)
                .width(Length::Fill);

            let (sink_slider, source_slider) = self
                .audio
                .as_ref()
                .map(|a| a.audio_sliders(self.sub_menu, opacity))
                .unwrap_or((None, None));

            let wifi_setting_button = self.network.as_ref().and_then(|n| {
                n.get_wifi_quick_setting_button(
                    id,
                    self.sub_menu,
                    config.wifi_more_cmd.is_some(),
                    opacity
                )
            });
            let quick_settings = quick_settings_section(
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
                                opacity
                            )
                        }),
                    self.network.as_ref().and_then(|n| {
                        n.get_vpn_quick_setting_button(
                            id,
                            self.sub_menu,
                            config.vpn_more_cmd.is_some(),
                            opacity
                        )
                    }),
                    self.network.as_ref().and_then(|n| {
                        if config.remove_airplane_btn {
                            None
                        } else {
                            Some(n.get_airplane_mode_quick_setting_button(opacity))
                        }
                    }),
                    self.idle_inhibitor.as_ref().and_then(|i| {
                        if config.remove_idle_btn {
                            None
                        } else {
                            Some((
                                quick_setting_button(
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
                        .and_then(|u| u.power_profile.get_quick_setting_button(opacity)),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
                opacity
            );

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
                                power_menu(opacity, config).map(Message::Power),
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
                                        opacity
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
                                        opacity
                                    ),
                                    opacity
                                )
                            })
                        })
                )
                .push_maybe(bottom_source_slider)
                .push_maybe(self.brightness.as_ref().map(|b| b.brightness_slider()))
                .push(quick_settings)
                .width(Length::Fill)
                .spacing(16)
                .into()
        }
    }
}
