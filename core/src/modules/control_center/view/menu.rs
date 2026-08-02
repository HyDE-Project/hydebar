//! Rendering of the settings menu contents.

use iced::{Element, Length, SurfaceId as Id, widget::Column};

use super::helpers::sub_menu_wrapper;
use crate::{
    components::{icons::IconTheme, push_maybe::PushMaybe, scale},
    config::{ControlCenterModuleConfig, Position},
    modules::control_center::{
        power::power_menu,
        state::{ControlCenter, Message, SubMenu}
    },
    password_dialog
};

mod header;
mod quick_settings;

impl ControlCenter {
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
