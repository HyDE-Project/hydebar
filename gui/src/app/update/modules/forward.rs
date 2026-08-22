//! The forwarding table handing each message to the module that owns it.
//!
//! Two rooms, by what the message is about: the compositor and the readings
//! it drives are here, and [`desktop`] takes what the user works the desktop
//! with — its look, its windows of its own, its bells.

mod desktop;

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
                self.history.saw(self.system_info.data());

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

                Task::perform(
                    hydebar_core::outputs::perch::strip_rows(),
                    Message::StripRowsMeasured
                )
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
            other => self.update_desktop_modules(other)
        }
    }
}
