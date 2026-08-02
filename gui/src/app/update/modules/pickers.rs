//! The wallpaper and layout pickers, whose windows wait for their listings.

use hydebar_core::modules;
use iced::Task;

use super::super::super::state::{App, Message};

impl App {
    /// Forwards a layout picker message, opening the waiting window on its
    /// roster.
    pub(super) fn update_bar_layout(
        &mut self,
        msg: modules::bar_layout::Message
    ) -> Task<Message> {
        let listed = matches!(msg, modules::bar_layout::Message::Listed(_));

        let task = self.bar_layout.update(msg).map(Message::BarLayout);

        if listed && let Some((id, button_ui_ref)) = self.bar_layout_pending.take() {
            if self.bar_layout.is_empty() {
                log::warn!("the desktop lists no bar layouts, the picker stays closed");

                return task;
            }

            if self.outputs.open_menu().is_some() {
                return task;
            }

            let open = self.outputs.toggle_menu(
                id,
                hydebar_core::menu::MenuType::BarLayout,
                button_ui_ref,
                &self.config
            );
            self.attend_the_open_menu();

            return Task::batch([task, open]);
        }

        task
    }

    /// Forwards a wallpaper picker message, opening the waiting window on its
    /// pictures.
    pub(super) fn update_wallpaper(&mut self, msg: modules::wallpaper::Message) -> Task<Message> {
        let config = std::sync::Arc::clone(&self.config);
        let listed = matches!(msg, modules::wallpaper::Message::Listed(_));

        let task = self.wallpaper.update(msg, &config).map(Message::Wallpaper);

        if listed && let Some((id, button_ui_ref)) = self.wallpaper_pending.take() {
            if self.wallpaper.is_empty() {
                log::warn!("the theme offers no wallpapers, the picker stays closed");

                return task;
            }

            if self.outputs.open_menu().is_some() {
                return task;
            }

            let open = self.outputs.toggle_menu(
                id,
                hydebar_core::menu::MenuType::Wallpaper,
                button_ui_ref,
                &self.config
            );
            self.attend_the_open_menu();

            return Task::batch([task, open]);
        }

        task
    }
}
