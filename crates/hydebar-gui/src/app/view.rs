use std::f32::consts::PI;

use hydebar_core::{
    HEIGHT,
    menu::{MenuLayout, MenuSize, MenuType, dismiss_area, menu_wrapper},
    modules::{
        control_center::{ControlCenterViewExt, audio::AudioMessage},
        custom_module
    },
    notifications_popup,
    outputs::HasOutput,
    style::{backdrop_color, darken_color},
    tooltip::tooltip_wrapper
};
use hydebar_proto::config::{AppearanceStyle, Position};
use iced::{
    Alignment, Color, Element, Gradient, Length, Radians, SurfaceId as Id, Theme,
    gradient::Linear,
    theme::Style,
    widget::{Row, container}
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
        self.theme_cache.clone()
    }

    /// Applies the fade of a travelling menu to its whole subtree.
    ///
    /// This is the one place any menu window animates from: the subtree is
    /// rethemed with every palette colour scaled to the travelled share, and
    /// the default text colour is restated from that faded palette so text
    /// without a colour of its own follows too. The views below are handed the
    /// resting opacity and never animate themselves, which is what makes the
    /// box, text, icons, buttons and swatches all move as one instead of the
    /// background dying before its content.
    /// The palette follows the cube of the travelled share on purpose. The
    /// box behind the content fades with an extra factor — the configured
    /// menu opacity — so on a linear palette the sliders and lists outlived
    /// their own window and hung in the air over the desktop as afterimages.
    /// Cubed, everything inside the window has left the screen while the
    /// window itself is still settling into the bar.
    fn faded_menu<'a>(&self, menu: Element<'a, Message>, progress: f32) -> Element<'a, Message> {
        if progress < 1.0 {
            iced::widget::themer(
                Some(hydebar_core::style::faded_theme(
                    &self.theme_cache,
                    progress * progress * progress
                )),
                menu
            )
            .text_color(|theme: &Theme| theme.palette().text)
            .into()
        } else {
            menu
        }
    }

    pub fn style(&self, theme: &Theme) -> Style {
        Style {
            background_color: Color::TRANSPARENT,
            text_color:       theme.palette().text
        }
    }

    pub fn scale_factor(&self, _id: Id) -> f64 {
        self.appearance().scale_factor
    }

    /// Theme facts a menu needs to place itself, at the given animated opacity.
    fn menu_layout(&self, opacity: f32, progress: f32) -> MenuLayout {
        MenuLayout {
            font_size: self.appearance().font_size_px(),
            bar_position: self.config.position,
            style: self.appearance().style,
            opacity,
            radius: self.appearance().pill_radius(),
            menu_backdrop: self.appearance().menu.backdrop,
            content_height: None,
            available_height: self.menu_room(),
            progress
        }
    }

    /// Height a menu box may take before its content has to scroll.
    ///
    /// Derived from the reported screen, never from a button viewport: the
    /// strip below the bar minus the box's own breathing room.
    fn menu_room(&self) -> Option<f32> {
        let bar = self.appearance().height.unwrap_or(HEIGHT as f32);

        self.screen_height
            .map(|screen| screen - bar - self.appearance().font_size_px() * 6.0)
            .filter(|room| *room > 0.0)
    }

    /// Placement of a menu whose content height the caller measured.
    fn measured_menu_layout(
        &self,
        opacity: f32,
        progress: f32,
        content_height: f32
    ) -> MenuLayout {
        MenuLayout {
            content_height: Some(content_height),
            ..self.menu_layout(opacity, progress)
        }
    }

    /// Content, width and measured height of the window `menu_type` opens.
    ///
    /// The one table naming what every menu shows: the wrapping, placement and
    /// fade around it are the same for all of them and live with the caller.
    /// [`None`] stands for a menu whose owner is gone, such as a custom module
    /// the configuration no longer declares.
    #[allow(clippy::type_complexity)]
    fn menu_page(
        &self,
        menu_type: &MenuType,
        id: Id,
        opacity: f32
    ) -> Option<(Element<'_, Message>, MenuSize, Option<f32>)> {
        match menu_type {
            MenuType::Updates => Some((
                self.updates
                    .menu_view(id, opacity, self.icons())
                    .map(Message::Updates),
                MenuSize::Small,
                None
            )),
            MenuType::Tray(name) => Some((
                self.tray
                    .menu_view(name, opacity, self.icons())
                    .map(Message::Tray),
                MenuSize::Small,
                None
            )),
            MenuType::ControlCenter => Some((
                self.control_center
                    .menu_view(
                        id,
                        &self.config.control_center,
                        opacity,
                        self.config.position,
                        self.icons()
                    )
                    .map(Message::ControlCenter),
                MenuSize::Medium,
                None
            )),
            MenuType::Audio => Some((
                iced::widget::mouse_area(
                    self.control_center
                        .audio_menu(
                            id,
                            &self.config.control_center,
                            opacity,
                            self.config.position,
                            self.icons()
                        )
                        .map(Message::ControlCenter)
                )
                .on_scroll(|delta| {
                    Message::ControlCenter(hydebar_core::modules::control_center::Message::Audio(
                        AudioMessage::SinkVolumeWheel(
                            hydebar_core::modules::control_center::audio::wheel_direction(delta)
                        )
                    ))
                })
                .into(),
                MenuSize::Medium,
                None
            )),
            MenuType::Network => Some((
                self.control_center
                    .network_menu(id, &self.config.control_center, opacity, self.icons())
                    .map(Message::ControlCenter),
                MenuSize::Medium,
                None
            )),
            MenuType::Bluetooth => Some((
                self.control_center
                    .bluetooth_menu(id, &self.config.control_center, opacity, self.icons())
                    .map(Message::ControlCenter),
                MenuSize::Medium,
                None
            )),
            MenuType::PowerProfile => Some((
                self.control_center
                    .power_profile_menu(opacity, &self.config.control_center, self.icons())
                    .map(Message::ControlCenter),
                MenuSize::Small,
                None
            )),
            MenuType::HydeMenu => Some((
                self.hyde_menu.menu_view(id, opacity).map(Message::HydeMenu),
                MenuSize::Small,
                None
            )),
            MenuType::Settings => {
                let metrics = self.settings.window_metrics(&self.config);

                Some((
                    self.settings
                        .menu_view(
                            &self.config,
                            opacity,
                            self.icons(),
                            self.magnification,
                            metrics.page_width
                        )
                        .map(Message::Settings),
                    MenuSize::Content(metrics.width),
                    Some(metrics.height)
                ))
            }
            MenuType::Themes => {
                let metrics = self.themes.window_metrics(&self.config);

                Some((
                    self.themes
                        .menu_view(&self.config, opacity, metrics.page_width)
                        .map(Message::Themes),
                    MenuSize::Content(metrics.width),
                    Some(metrics.height)
                ))
            }
            MenuType::MediaPlayer => Some((
                self.media_player
                    .menu_view(&self.config.media_player, opacity, self.icons())
                    .map(Message::MediaPlayer),
                MenuSize::Large,
                None
            )),
            MenuType::SystemInfo => Some((
                self.system_info
                    .menu_view(&self.config.system, self.icons())
                    .map(Message::SystemInfo),
                MenuSize::Content(
                    hydebar_core::modules::system_info::SystemInfo::content_width(
                        self.appearance().font_size_px()
                    )
                ),
                Some(self.system_info.content_height(&self.config.system))
            )),
            MenuType::Cpu => Some((
                self.system_info
                    .cpu_menu_view(self.icons())
                    .map(Message::SystemInfo),
                MenuSize::Content(
                    hydebar_core::modules::system_info::SystemInfo::content_width(
                        self.appearance().font_size_px()
                    )
                ),
                Some(self.system_info.cpu_content_height())
            )),
            MenuType::Memory => Some((
                self.system_info
                    .memory_menu_view(self.icons())
                    .map(Message::SystemInfo),
                MenuSize::Content(
                    hydebar_core::modules::system_info::SystemInfo::content_width(
                        self.appearance().font_size_px()
                    )
                ),
                Some(self.system_info.memory_content_height())
            )),
            MenuType::CpuTemp => Some((
                self.system_info
                    .cpu_temp_menu_view(self.icons())
                    .map(Message::SystemInfo),
                MenuSize::Content(
                    hydebar_core::modules::system_info::SystemInfo::content_width(
                        self.appearance().font_size_px()
                    )
                ),
                Some(self.system_info.cpu_temp_content_height())
            )),
            MenuType::Gpu => Some((
                self.system_info
                    .gpu_menu_view(self.icons())
                    .map(Message::SystemInfo),
                MenuSize::Content(
                    hydebar_core::modules::system_info::SystemInfo::content_width(
                        self.appearance().font_size_px()
                    )
                ),
                Some(self.system_info.gpu_content_height())
            )),
            MenuType::Notifications => Some((
                self.notifications
                    .menu_view(opacity, self.icons())
                    .map(Message::Notifications),
                MenuSize::Medium,
                None
            )),
            MenuType::Screenshot => Some((
                self.screenshot
                    .menu_view(opacity, self.icons())
                    .map(Message::Screenshot),
                MenuSize::Small,
                None
            )),
            MenuType::Calendar => Some((
                self.calendar.menu_view(self.icons()).map(Message::Calendar),
                MenuSize::Content(hydebar_core::modules::calendar::Calendar::content_width(
                    self.appearance().font_size_px()
                )),
                Some(hydebar_core::modules::calendar::Calendar::content_height())
            )),
            MenuType::Custom(name) => self
                .config
                .custom_modules
                .iter()
                .find(|definition| &definition.name == name)
                .map(|definition| {
                    (
                        custom_module::menu_view(definition, self.appearance(), opacity, {
                            move |entry| custom_menu_message(id, entry)
                        }),
                        MenuSize::Small,
                        None
                    )
                })
        }
    }

    /// The greeting shown mid-screen while the bar comes up.
    ///
    /// Drawn on the idle menu surface, which spans the screen and is raised
    /// for exactly the greeting's lifetime; it breathes in with the bar's
    /// birth and lets itself out three seconds later. An empty row whenever
    /// the greeting is over, disabled, or animations are off.
    fn screen_greeting(&self) -> Element<'_, Message> {
        let presence = self.greeting.value().clamp(0.0, 1.0);

        if presence <= 0.004 {
            return Row::new().into();
        }

        let line = iced::widget::text(self.greeting_line.as_str())
            .size(self.appearance().font_size_px() * 2.4)
            .color(self.theme_cache.palette().text.scale_alpha(presence));

        container(line)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
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
                let left = self.modules_section(
                    &self.config.modules.left,
                    id,
                    self.appearance().opacity,
                    0
                );
                let center = self.modules_section(
                    &self.config.modules.center,
                    id,
                    self.appearance().opacity,
                    self.config.modules.left.len()
                );
                let right = self.modules_section(
                    &self.config.modules.right,
                    id,
                    self.appearance().opacity,
                    self.config.modules.left.len() + self.config.modules.center.len()
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
                let menu_opacity = self.config.appearance.menu.opacity;
                let menu_progress = self.outputs.get_menu_progress(id);

                let menu = menu_info.and_then(|(menu_type, button_ui_ref)| {
                    self.menu_page(menu_type, id, menu_opacity).map(
                        |(content, size, measured_height)| {
                            let layout = match measured_height {
                                Some(height) => {
                                    self.measured_menu_layout(menu_opacity, menu_progress, height)
                                }
                                None => self.menu_layout(menu_opacity, menu_progress)
                            };

                            menu_wrapper(
                                id,
                                content,
                                size,
                                *button_ui_ref,
                                layout,
                                Message::None,
                                Message::CloseMenu(id)
                            )
                        }
                    )
                });

                match menu {
                    Some(menu) => self.faded_menu(menu, menu_progress),
                    None => self.screen_greeting()
                }
            }
            Some(HasOutput::Notifications) => notifications_popup::view(
                &self.notification_popups,
                self.appearance(),
                self.config.position
            ),
            Some(HasOutput::Tooltip) => match self.outputs.tooltip(id) {
                Some(info) => self.faded_menu(
                    tooltip_wrapper(info, self.config.position, self.appearance()),
                    self.hints.presence()
                ),
                None => Row::new().into()
            },
            None => Row::new().into()
        }
    }
}
