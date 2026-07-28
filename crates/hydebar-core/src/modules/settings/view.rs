use iced::{Element, window::Id};

use super::state::{Message, Settings};
use crate::{
    components::icons::IconTheme,
    config::{Position, SettingsModuleConfig},
    modules::OnModulePress
};

mod bar;
mod helpers;
mod menu;
mod quick_button;

#[cfg(test)]
mod tests;

pub use quick_button::quick_setting_button;

pub trait SettingsViewExt {
    type ViewData<'a>;

    fn settings_view<M>(
        &self,
        data: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>;

    fn menu_view(
        &self,
        id: Id,
        config: &SettingsModuleConfig,
        opacity: f32,
        position: Position,
        icons: &IconTheme
    ) -> Element<'_, Message>;
}

impl SettingsViewExt for Settings {
    type ViewData<'a> = &'a IconTheme;

    fn settings_view<M>(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        self.render_bar(icons)
    }

    fn menu_view(
        &self,
        id: Id,
        config: &SettingsModuleConfig,
        opacity: f32,
        position: Position,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        self.render_menu(id, config, opacity, position, icons)
    }
}
