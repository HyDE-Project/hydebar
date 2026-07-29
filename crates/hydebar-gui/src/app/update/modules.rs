//! Messages forwarded to individual bar modules.

use std::sync::Arc;

use hydebar_core::{
    menu::MenuType,
    modules::{self, tray::TrayMessage},
    services::{ServiceEvent, tray::TrayEvent},
    utils
};
use iced::Task;
use log::error;

use super::super::state::{App, Message};

impl App {
    /// Handles the messages this module owns.
    pub(super) fn update_modules(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Updates(message) => {
                if let Some(updates_config) = self.config.updates.as_ref() {
                    self.updates
                        .update(message, updates_config, &mut self.outputs, &self.config);
                }
                Task::none()
            }
            Message::OpenLauncher => {
                if let Some(app_launcher_cmd) = self.config.app_launcher_cmd.as_ref() {
                    utils::launcher::execute_command(app_launcher_cmd.to_string());
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
                match self.custom.get_mut(&name) {
                    Some(c) => c.update(message),
                    None => error!("Custom module '{name}' not found")
                };
                Task::none()
            }
            Message::OpenClipboard => {
                if let Some(clipboard_cmd) = self.config.clipboard_cmd.as_ref() {
                    utils::launcher::execute_command(clipboard_cmd.to_string());
                }
                Task::none()
            }
            Message::Workspaces(msg) => {
                self.workspaces.update(msg, &self.config.workspaces);

                Task::none()
            }
            Message::WindowTitle(message) => {
                self.window_title.update(message, &self.config.window_title);
                Task::none()
            }
            Message::SystemInfo(message) => {
                self.system_info.update(message, &self.config.system);
                Task::none()
            }
            Message::KeyboardLayout(message) => {
                self.keyboard_layout.update(message);
                Task::none()
            }
            Message::KeyboardSubmap(message) => {
                self.keyboard_submap.update(message);
                Task::none()
            }
            Message::Tray(msg) => {
                let close_tray = match &msg {
                    TrayMessage::Event(event) => {
                        if let ServiceEvent::Update(TrayEvent::Unregistered(name)) = event.as_ref()
                        {
                            self.outputs
                                .close_all_menu_if(MenuType::Tray(name.clone()), &self.config)
                        } else {
                            Task::none()
                        }
                    }
                    _ => Task::none()
                };

                self.tray.update(msg);
                close_tray
            }
            Message::Clock(message) => {
                self.clock.update(message, &self.config.clock);
                Task::none()
            }
            Message::Weather(message) => {
                self.weather.update(message.clone());

                // If clock is configured to show weather, update it too
                if self.config.clock.show_weather
                    && let modules::weather::Message::Update(weather_data) = message
                {
                    self.clock.update(
                        modules::clock::Message::UpdateWeather(weather_data),
                        &self.config.clock
                    );
                }

                Task::none()
            }
            Message::Battery(message) => {
                self.battery.update(message);
                Task::none()
            }
            Message::Privacy(msg) => {
                self.privacy.update(msg);
                Task::none()
            }
            Message::ControlCenter(message) => {
                self.control_center.update(
                    message,
                    &self.config.control_center,
                    &mut self.outputs,
                    &self.config
                );
                Task::none()
            }
            Message::Settings(msg) => {
                let config = Arc::clone(&self.config);
                self.settings.update(msg, &config);
                Task::none()
            }
            Message::MediaPlayer(msg) => {
                self.media_player.update(msg);
                Task::none()
            }
            Message::Notifications(msg) => {
                self.notifications.update(msg);
                Task::none()
            }
            Message::Screenshot(msg) => {
                self.screenshot.update(msg);
                Task::none()
            }
            _ => Task::none()
        }
    }
}
