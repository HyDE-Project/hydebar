//! The menus of what the session has to say: bells, updates, the tray.

use hydebar_core::{
    menu::{MenuSize, MenuType},
    modules::custom_module
};
use iced::SurfaceId as Id;

use super::{
    super::super::{
        modules::actions::custom_menu_message,
        state::{App, Message}
    },
    Page
};

impl App {
    /// What one of the session's own menus shows.
    ///
    /// [`None`] for a menu this table does not own, which the caller never
    /// asks it for.
    pub(super) fn notice_page(
        &self,
        menu_type: &MenuType,
        id: Id,
        opacity: f32
    ) -> Option<Page<'_>> {
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

            MenuType::MediaPlayer => Some((
                self.media_player
                    .menu_view(&self.config.media_player, opacity, self.icons())
                    .map(Message::MediaPlayer),
                MenuSize::Large,
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

            _ => None
        }
    }

    /// What the menu of the custom module named `name` shows.
    ///
    /// [`None`] when the configuration no longer declares it.
    pub(super) fn custom_page(&self, name: &str, id: Id, opacity: f32) -> Option<Page<'_>> {
        self.config
            .custom_modules
            .iter()
            .find(|definition| definition.name == name)
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
