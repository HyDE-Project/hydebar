//! What the user works the desktop with: its look, its windows, its bells.

use std::sync::Arc;

use iced::Task;

use super::super::super::super::state::{App, Message};

impl App {
    /// Handles the messages the desktop's own modules own.
    pub(super) fn update_desktop_modules(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Settings(msg) => {
                let config = Arc::clone(&self.config);

                self.settings.update(msg, &config).map(Message::Settings)
            }
            Message::Themes(msg) => {
                let config = Arc::clone(&self.config);

                Task::batch([
                    self.themes.update(msg, &config).map(Message::Themes),
                    crate::app::update::outputs::read_wallpaper()
                ])
            }
            Message::BarLayout(msg) => self.update_bar_layout(msg),
            Message::Wallpaper(msg) => Task::batch([
                self.update_wallpaper(msg),
                crate::app::update::outputs::read_wallpaper()
            ]),
            Message::MediaPlayer(msg) => {
                self.media_player.update(msg, &self.config.media_player);
                Task::none()
            }
            Message::ExpirePopups => self.expire_popups(),
            Message::Notifications(msg) => self.update_notifications(msg),
            Message::Screenshot(msg) => {
                self.screenshot.update(msg);
                Task::none()
            }
            _ => Task::none()
        }
    }
}
