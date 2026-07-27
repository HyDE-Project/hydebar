//! Keyboard navigation across modules.

use hydebar_core::{modules::OnModulePress, position_button::ButtonUIRef};
use iced::Task;
use log::{debug, info};

use super::super::state::{App, Message};

impl App {
    /// Handles the messages this module owns.
    pub(super) fn update_navigation(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ActivateNavigationMode => {
                if !self.navigation_mode && self.config.keybindings.enabled {
                    info!("Activating navigation mode");
                    self.navigation_mode = true;
                    self.focused_module_index = Some(0);
                }
                Task::none()
            }
            Message::DeactivateNavigationMode => {
                if self.navigation_mode {
                    info!("Deactivating navigation mode");
                    self.navigation_mode = false;
                    self.focused_module_index = None;
                }
                if self.outputs.menu_is_open() {
                    self.outputs.close_all_menus(&self.config)
                } else {
                    Task::none()
                }
            }
            Message::NavigateUp | Message::NavigateDown => {
                if !self.navigation_mode {
                    return Task::none();
                }

                Task::none()
            }
            Message::NavigateLeft => {
                if !self.navigation_mode {
                    return Task::none();
                }

                if let Some(current) = self.focused_module_index
                    && current > 0
                {
                    self.focused_module_index = Some(current - 1);
                    debug!("Navigate left: focus moved to module {}", current - 1);
                }
                Task::none()
            }
            Message::NavigateRight => {
                if !self.navigation_mode {
                    return Task::none();
                }

                if let Some(current) = self.focused_module_index {
                    let all_modules = self.get_all_modules_count();
                    if current + 1 < all_modules {
                        self.focused_module_index = Some(current + 1);
                        debug!("Navigate right: focus moved to module {}", current + 1);
                    }
                }
                Task::none()
            }
            Message::ActivateFocusedModule => {
                if !self.navigation_mode || self.focused_module_index.is_none() {
                    return Task::none();
                }

                let index = self.focused_module_index.unwrap();

                let main_window_id = if let Some(id) = self.outputs.first_main_window_id() {
                    id
                } else {
                    return Task::none();
                };

                if let Some(action) = self.get_module_at_index(index, main_window_id) {
                    match action {
                        OnModulePress::Action(msg) => {
                            info!("Activating module at index {} with action", index);
                            return self.update(*msg);
                        }
                        OnModulePress::ToggleMenu(menu_type) => {
                            info!(
                                "Activating module at index {} - opening menu {:?}",
                                index, menu_type
                            );

                            let center_button_ref = ButtonUIRef {
                                position: iced::Point {
                                    x: 960.0, y: 20.0
                                },
                                viewport: (1920.0, 1080.0)
                            };

                            return self.update(Message::ToggleMenu(
                                menu_type,
                                main_window_id,
                                center_button_ref
                            ));
                        }
                    }
                }

                Task::none()
            }
            _ => Task::none()
        }
    }
}
