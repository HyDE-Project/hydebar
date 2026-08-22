//! The menus of the control centre: the machine's own switches.

use hydebar_core::{
    menu::{MenuSize, MenuType},
    modules::control_center::{ControlCenterViewExt, audio::AudioMessage}
};
use iced::SurfaceId as Id;

use super::{
    super::super::state::{App, Message},
    Page
};

impl App {
    /// What one of the control centre's own menus shows.
    ///
    /// [`None`] for a menu this table does not own, which the caller never
    /// asks it for.
    pub(super) fn control_centre_page(
        &self,
        menu_type: &MenuType,
        id: Id,
        opacity: f32
    ) -> Option<Page<'_>> {
        match menu_type {
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
            _ => None
        }
    }
}
