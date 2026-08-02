//! The table naming what every menu window shows.

use hydebar_core::{
    menu::{MenuSize, MenuType},
    modules::{
        control_center::{ControlCenterViewExt, audio::AudioMessage},
        custom_module
    }
};
use iced::{Element, SurfaceId as Id};

use super::super::{
    modules::actions::custom_menu_message,
    state::{App, Message}
};

impl App {
    /// Content, width and measured height of the window `menu_type` opens.
    ///
    /// The one table naming what every menu shows: the wrapping, placement and
    /// fade around it are the same for all of them and live with the caller.
    /// [`None`] stands for a menu whose owner is gone, such as a custom module
    /// the configuration no longer declares.
    #[allow(clippy::type_complexity)]
    #[expect(
        clippy::too_many_lines,
        reason = "one table naming what every menu shows, one arm per menu"
    )]
    pub(super) fn menu_page(
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
            MenuType::MediaPlayer => Some((
                self.media_player
                    .menu_view(&self.config.media_player, opacity, self.icons())
                    .map(Message::MediaPlayer),
                MenuSize::Large,
                None
            )),
            MenuType::Wallpaper => Some((
                self.wallpaper
                    .menu_view(self.appearance().font_size_px())
                    .map(Message::Wallpaper),
                MenuSize::Medium,
                None
            )),
            MenuType::BarLayout => Some((
                self.bar_layout
                    .menu_view(self.appearance().font_size_px())
                    .map(Message::BarLayout),
                MenuSize::Small,
                None
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
            MenuType::Settings
            | MenuType::Themes
            | MenuType::SystemInfo
            | MenuType::Cpu
            | MenuType::Memory
            | MenuType::CpuTemp
            | MenuType::Gpu
            | MenuType::Calendar => self.measured_page(menu_type, opacity),
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
}
