//! Opening and closing menu surfaces.

use hydebar_core::{
    menu::MenuType,
    modules::{self, settings::brightness::BrightnessMessage},
    services::brightness::BrightnessCommand
};
use iced::Task;

use super::super::state::{App, Message};

impl App {
    /// Handles the messages this module owns.
    pub(super) fn update_menus(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleMenu(menu_type, id, button_ui_ref) => {
                let mut cmd = vec![];
                match &menu_type {
                    MenuType::Updates => {
                        self.updates.is_updates_list_open = false;
                    }
                    MenuType::Tray(name) => {
                        if let Some(_tray) = self
                            .tray
                            .service
                            .as_ref()
                            .and_then(|t| t.iter().find(|t| &t.name == name))
                        {
                            self.tray.submenus.clear();
                        }
                    }
                    MenuType::Settings => {
                        self.settings.sub_menu = None;

                        if let Some(brightness) = self.settings.brightness.as_mut() {
                            use hydebar_core::services::Service;
                            cmd.push(brightness.command(BrightnessCommand::Refresh).map(
                                |event| {
                                    Message::Settings(modules::settings::Message::Brightness(
                                        BrightnessMessage::Event(event)
                                    ))
                                }
                            ));
                        }
                    }
                    _ => {}
                };
                cmd.push(
                    self.outputs
                        .toggle_menu(id, menu_type, button_ui_ref, &self.config)
                );

                Task::batch(cmd)
            }
            Message::ModuleTooltip(id, Some(info)) => self.outputs.show_tooltip(id, info),
            Message::ModuleTooltip(id, None) => self.outputs.hide_tooltip(id),
            Message::CloseMenu(id) => self.outputs.close_menu(id, &self.config),
            Message::CloseAllMenus => {
                if self.outputs.menu_is_open() {
                    self.outputs.close_all_menus(&self.config)
                } else {
                    Task::none()
                }
            }
            _ => Task::none()
        }
    }
}
