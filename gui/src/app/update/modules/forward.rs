//! The forwarding table handing each message to the module that owns it.

use std::sync::Arc;

use hydebar_core::{
    menu::MenuType,
    modules::tray::TrayMessage,
    services::{ServiceEvent, tray::TrayEvent}
};
use iced::Task;

use super::super::super::state::{App, Message};

impl App {
    /// Handles the messages this module owns.
    #[expect(
        clippy::too_many_lines,
        reason = "one dispatch arm per module message, read as a single table"
    )]
    pub(crate) fn update_modules(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Updates(message) => {
                if let Some(updates_config) = self.config.updates.as_ref() {
                    self.updates
                        .update(message, updates_config, &mut self.outputs, &self.config);
                }
                Task::none()
            }
            Message::OpenLauncher
            | Message::OpenClipboard
            | Message::LaunchCommand(_)
            | Message::CustomMenuAction(..)
            | Message::CustomUpdate(..) => self.update_commands(message),
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
                self.keyboard_layout.update(
                    message,
                    &self.config.keyboard_layout,
                    self.config.appearance.animations.enabled
                );
                Task::none()
            }
            Message::KeyboardSubmap(message) => {
                self.keyboard_submap
                    .update(message, self.config.appearance.animations.enabled);
                Task::none()
            }
            Message::Taskbar(msg) => {
                self.taskbar.update(msg);
                Task::none()
            }
            Message::Desk(msg) => {
                self.desk.update(msg);
                self.unfold_desk();
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
                self.clock.update(
                    message,
                    &self.config.clock,
                    self.config.appearance.animations.enabled
                );
                Task::none()
            }
            Message::Calendar(message) => {
                self.calendar.update(message);
                Task::none()
            }
            Message::HydeMenu(message) => match self.hyde_menu.update(message) {
                Some((surface, command)) => {
                    hydebar_core::utils::launcher::execute_command(command);

                    self.outputs.close_menu(surface, &self.config)
                }
                None => Task::none()
            },
            Message::Weather(message) => {
                self.weather.update(message);
                Task::none()
            }
            Message::Battery(message) => {
                self.battery
                    .update(message, self.config.appearance.animations.enabled);
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

                self.settings.update(msg, &config).map(Message::Settings)
            }
            Message::Themes(msg) => {
                let config = Arc::clone(&self.config);

                self.themes.update(msg, &config).map(Message::Themes)
            }
            Message::BarLayout(msg) => self.update_bar_layout(msg),
            Message::Wallpaper(msg) => self.update_wallpaper(msg),
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
