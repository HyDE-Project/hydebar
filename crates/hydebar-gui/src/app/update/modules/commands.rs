//! Commands a bar press launches, and custom module updates.

use hydebar_core::utils;
use iced::Task;
use log::error;

use super::super::super::state::{App, Message};

impl App {
    /// Handles the messages that launch a command or feed a custom module.
    pub(super) fn update_commands(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenLauncher => {
                if let Some(app_launcher_cmd) = self.config.app_launcher_cmd.as_ref() {
                    utils::launcher::execute_command(app_launcher_cmd.clone());
                }
                Task::none()
            }
            Message::OpenClipboard => {
                if let Some(clipboard_cmd) = self.config.clipboard_cmd.as_ref() {
                    utils::launcher::execute_command(clipboard_cmd.clone());
                }
                Task::none()
            }
            Message::LaunchCommand(command) => {
                utils::launcher::execute_command(command);
                Task::none()
            }
            Message::CustomMenuAction(id, command) => {
                utils::launcher::execute_command(command);
                self.outputs.close_menu(id, &self.config)
            }
            Message::CustomUpdate(name, message) => {
                match self.custom.get_mut(name.as_ref()) {
                    Some(c) => c.update(message),
                    None => error!("Custom module '{name}' not found")
                }
                Task::none()
            }
            _ => Task::none()
        }
    }
}
