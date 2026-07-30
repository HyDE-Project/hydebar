use std::f32::consts::PI;

use hydebar_core::{
    HEIGHT,
    menu::{MenuLayout, MenuSize, MenuType, dismiss_area, menu_wrapper},
    modules::{control_center::ControlCenterViewExt, custom_module},
    notifications_popup,
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
            radius: self.appearance().pill_radius(),
            menu_backdrop: self.appearance().menu.backdrop,
            content_height: None
        }
    }

    /// Placement of a menu whose content height the caller measured.
    fn measured_menu_layout(&self, opacity: f32, content_height: f32) -> MenuLayout {
        MenuLayout {
            content_height: Some(content_height),
            ..self.menu_layout(opacity)
        }
    }

    /// Wraps the bar so a press on it takes the open menu down.
    ///
    /// The menu backdrop covers the screen the bar leaves free and nothing
    /// else, so the bar is the one place the rule that a press outside a menu
    /// dismisses it has to be applied from. The wrapper is only there while a
    /// menu is open, so an ordinary press on the bar costs nothing.
    fn dismisses_the_open_menu<'a>(&self, bar: Element<'a, Message>) -> Element<'a, Message> {
        if self.outputs.menu_is_open() {
            dismiss_area(bar, Message::BarPressed, Message::BarReleased).into()
        } else {
            bar
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

                let bar = container(centerbox).style(|t| container::Style {
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
                            let wash = (self.appearance().bar_opacity > 0.0).then(|| {
                                t.palette()
                                    .background
                                    .scale_alpha(self.appearance().bar_opacity)
                            });

                            // The dim of an open menu lands on top of the
                            // strip's own wash, never in its place: the wash
                            // is what the compositor's blur threshold sees,
                            // and replacing it with a fully clear backdrop
                            // turned the blur off for as long as a menu was
                            // open.
                            match (wash, self.outputs.menu_is_open()) {
                                (Some(wash), true) => Some(
                                    darken_color(wash, self.appearance().menu.backdrop).into()
                                ),
                                (Some(wash), false) => Some(wash.into()),
                                (None, true) => {
                                    Some(backdrop_color(self.appearance().menu.backdrop).into())
                                }
                                (None, false) => None
                            }
                        }
                    },
                    ..Default::default()
                });

                self.dismisses_the_open_menu(bar.into())
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
                            .menu_view(
                                &self.config,
                                animated_opacity,
                                self.icons(),
                                self.magnification
                            )
                            .map(Message::Settings),
                        MenuSize::Content(self.settings.content_width(&self.config)),
                        *button_ui_ref,
                        self.measured_menu_layout(
                            animated_opacity,
                            self.settings.content_height(&self.config)
                        ),
                        Message::None,
                        Message::CloseMenu(id)
                    ),
                    Some((MenuType::Themes, button_ui_ref)) => menu_wrapper(
                        id,
                        self.themes
                            .menu_view(&self.config, animated_opacity)
                            .map(Message::Themes),
                        MenuSize::Content(self.themes.content_width(&self.config)),
                        *button_ui_ref,
                        self.measured_menu_layout(
                            animated_opacity,
                            self.themes.content_height(&self.config)
                        ),
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
                            .menu_view(&self.config.system, self.icons())
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
            Some(HasOutput::Notifications) => notifications_popup::view(
                &self.notification_popups,
                self.appearance(),
                self.config.position
            ),
            Some(HasOutput::Tooltip) => match self.outputs.tooltip(id) {
                Some(info) => tooltip_wrapper(info, self.config.position, self.appearance()),
                None => Row::new().into()
            },
            None => Row::new().into()
        }
    }
}
