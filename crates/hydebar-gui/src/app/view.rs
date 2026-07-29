use std::f32::consts::PI;

use hydebar_core::{
    HEIGHT,
    menu::{MenuLayout, MenuSize, MenuType, menu_wrapper},
    modules::{control_center::ControlCenterViewExt, custom_module},
    outputs::HasOutput,
    style::{backdrop_color, darken_color, hydebar_theme},
    tooltip::tooltip_wrapper
};
use hydebar_proto::config::{AppearanceStyle, Position};
use iced::{
    Alignment, Color, Element, Gradient, Length, Radians, Theme,
    gradient::Linear,
    theme::Style,
    widget::{Row, container},
    window::Id
};

use super::{
    modules::actions::custom_menu_message,
    state::{App, Message}
};
use crate::centerbox;

impl App {
    pub fn title(&self, _id: Id) -> String {
        String::from("hydebar")
    }

    pub fn theme(&self, _id: Id) -> Theme {
        hydebar_theme(self.appearance())
    }

    pub fn style(&self, theme: &Theme) -> Style {
        Style {
            background_color: Color::TRANSPARENT,
            text_color:       theme.palette().text,
            icon_color:       theme.palette().text
        }
    }

    pub fn scale_factor(&self, _id: Id) -> f64 {
        self.appearance().scale_factor
    }

    /// Theme facts a menu needs to place itself, at the given animated opacity.
    fn menu_layout(&self, opacity: f32) -> MenuLayout {
        MenuLayout {
            font_size: self.appearance().font_size_px(),
            bar_position: self.config.position,
            style: self.appearance().style,
            opacity,
            menu_backdrop: self.appearance().menu.backdrop
        }
    }

    pub fn view(&self, id: Id) -> Element<'_, Message> {
        match self.outputs.has(id) {
            Some(HasOutput::Main) => {
                let left =
                    self.modules_section(&self.config.modules.left, id, self.appearance().opacity);
                let center = self.modules_section(
                    &self.config.modules.center,
                    id,
                    self.appearance().opacity
                );
                let right = self.modules_section(
                    &self.config.modules.right,
                    id,
                    self.appearance().opacity
                );

                let bar_height = self.appearance().height.unwrap_or(HEIGHT as f32);

                let centerbox = centerbox::Centerbox::new([left, center, right])
                    .spacing(self.appearance().group_gap())
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .height(if self.appearance().style == AppearanceStyle::Islands {
                        bar_height
                    } else {
                        bar_height - 8.
                    })
                    .padding(if self.appearance().style == AppearanceStyle::Islands {
                        self.appearance().bar_padding()
                    } else {
                        [0.0, 0.0]
                    });

                container(centerbox)
                    .style(|t| container::Style {
                        background: match self.appearance().style {
                            AppearanceStyle::Gradient => Some({
                                let start_color = t
                                    .palette()
                                    .background
                                    .scale_alpha(self.appearance().opacity);

                                let start_color = if self.outputs.menu_is_open() {
                                    darken_color(start_color, self.appearance().menu.backdrop)
                                } else {
                                    start_color
                                };

                                let end_color = if self.outputs.menu_is_open() {
                                    backdrop_color(self.appearance().menu.backdrop)
                                } else {
                                    Color::TRANSPARENT
                                };

                                Gradient::Linear(
                                    Linear::new(Radians(PI))
                                        .add_stop(
                                            0.0,
                                            match self.config.position {
                                                Position::Top => start_color,
                                                Position::Bottom => end_color
                                            }
                                        )
                                        .add_stop(
                                            1.0,
                                            match self.config.position {
                                                Position::Top => end_color,
                                                Position::Bottom => start_color
                                            }
                                        )
                                )
                                .into()
                            }),
                            AppearanceStyle::Solid => Some({
                                let bg = t
                                    .palette()
                                    .background
                                    .scale_alpha(self.appearance().opacity);
                                if self.outputs.menu_is_open() {
                                    darken_color(bg, self.appearance().menu.backdrop)
                                } else {
                                    bg
                                }
                                .into()
                            }),
                            AppearanceStyle::Islands => {
                                if self.outputs.menu_is_open() {
                                    Some(backdrop_color(self.appearance().menu.backdrop).into())
                                } else if self.appearance().bar_opacity > 0.0 {
                                    Some(
                                        t.palette()
                                            .background
                                            .scale_alpha(self.appearance().bar_opacity)
                                            .into()
                                    )
                                } else {
                                    None
                                }
                            }
                        },
                        ..Default::default()
                    })
                    .into()
            }
            Some(HasOutput::Menu(menu_info)) => {
                let animated_opacity = self.outputs.get_menu_opacity(id);
                match menu_info {
                    Some((MenuType::Updates, button_ui_ref)) => menu_wrapper(
                        id,
                        self.updates
                            .menu_view(id, animated_opacity, self.icons())
                            .map(Message::Updates),
                        MenuSize::Small,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::Tray(name), button_ui_ref)) => menu_wrapper(
                        id,
                        self.tray
                            .menu_view(name, animated_opacity, self.icons())
                            .map(Message::Tray),
                        MenuSize::Small,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::ControlCenter, button_ui_ref)) => menu_wrapper(
                        id,
                        self.control_center
                            .menu_view(
                                id,
                                &self.config.control_center,
                                animated_opacity,
                                self.config.position,
                                self.icons()
                            )
                            .map(Message::ControlCenter),
                        MenuSize::Medium,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::Audio, button_ui_ref)) => menu_wrapper(
                        id,
                        self.control_center
                            .audio_menu(
                                id,
                                &self.config.control_center,
                                animated_opacity,
                                self.config.position,
                                self.icons()
                            )
                            .map(Message::ControlCenter),
                        MenuSize::Medium,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::Network, button_ui_ref)) => menu_wrapper(
                        id,
                        self.control_center
                            .network_menu(
                                id,
                                &self.config.control_center,
                                animated_opacity,
                                self.icons()
                            )
                            .map(Message::ControlCenter),
                        MenuSize::Medium,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::Bluetooth, button_ui_ref)) => menu_wrapper(
                        id,
                        self.control_center
                            .bluetooth_menu(
                                id,
                                &self.config.control_center,
                                animated_opacity,
                                self.icons()
                            )
                            .map(Message::ControlCenter),
                        MenuSize::Medium,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::PowerProfile, button_ui_ref)) => menu_wrapper(
                        id,
                        self.control_center
                            .power_profile_menu(
                                animated_opacity,
                                &self.config.control_center,
                                self.icons()
                            )
                            .map(Message::ControlCenter),
                        MenuSize::Small,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::Settings, button_ui_ref)) => menu_wrapper(
                        id,
                        self.settings
                            .menu_view(&self.config, animated_opacity, self.icons())
                            .map(Message::Settings),
                        MenuSize::Wide,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::MediaPlayer, button_ui_ref)) => menu_wrapper(
                        id,
                        self.media_player
                            .menu_view(&self.config.media_player, animated_opacity, self.icons())
                            .map(Message::MediaPlayer),
                        MenuSize::Large,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::SystemInfo, button_ui_ref)) => menu_wrapper(
                        id,
                        self.system_info
                            .menu_view(self.icons())
                            .map(Message::SystemInfo),
                        MenuSize::Medium,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::Notifications, button_ui_ref)) => menu_wrapper(
                        id,
                        self.notifications
                            .menu_view(animated_opacity, self.icons())
                            .map(Message::Notifications),
                        MenuSize::Medium,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::Screenshot, button_ui_ref)) => menu_wrapper(
                        id,
                        self.screenshot
                            .menu_view(animated_opacity)
                            .map(Message::Screenshot),
                        MenuSize::Small,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::Calendar, button_ui_ref)) => menu_wrapper(
                        id,
                        self.clock.menu_view(self.icons()).map(Message::Clock),
                        MenuSize::Medium,
                        *button_ui_ref,
                        self.menu_layout(animated_opacity),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::Custom(name), button_ui_ref)) => {
                        match self
                            .config
                            .custom_modules
                            .iter()
                            .find(|definition| &definition.name == name)
                        {
                            Some(definition) => menu_wrapper(
                                id,
                                custom_module::menu_view(
                                    definition,
                                    self.appearance(),
                                    animated_opacity,
                                    move |entry| custom_menu_message(id, entry)
                                ),
                                MenuSize::Small,
                                *button_ui_ref,
                                self.menu_layout(animated_opacity),
                                Message::None,
                                Message::CloseMenu(id)
                            ),
                            None => Row::new().into()
                        }
                    }
                    None => Row::new().into()
                }
            }
            Some(HasOutput::Tooltip) => match self.outputs.tooltip(id) {
                Some(info) => tooltip_wrapper(info, self.config.position, self.appearance()),
                None => Row::new().into()
            },
            None => Row::new().into()
        }
    }
}
