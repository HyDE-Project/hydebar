//! The menus of the desk itself: its look, its wallpaper, its layout.

use hydebar_core::menu::{MenuSize, MenuType};
use iced::SurfaceId as Id;

use super::{
    super::super::state::{App, Message},
    Page
};

impl App {
    /// What one of the desk's own menus shows.
    ///
    /// [`None`] for a menu this table does not own, which the caller never
    /// asks it for.
    pub(super) fn desktop_page(
        &self,
        menu_type: &MenuType,
        id: Id,
        opacity: f32
    ) -> Option<Page<'_>> {
        match menu_type {
            MenuType::HydeMenu => Some((
                self.hyde_menu.menu_view(id, opacity).map(Message::HydeMenu),
                MenuSize::Small,
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

            _ => None
        }
    }
}
