//! Dispatch of the menu messages: hovers, toggles and dismissals.

use iced::Task;

use super::super::super::state::{App, Message};

impl App {
    /// Handles the messages this module owns.
    pub(crate) fn update_menus(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ModuleHover {
                surface,
                module,
                entered,
                tooltip
            } => self.on_module_hover(surface, module, entered, tooltip),
            Message::ToggleMenu(menu_type, id, button_ui_ref) => {
                self.on_toggle_menu(menu_type, id, button_ui_ref)
            }
            Message::BarPressed => {
                self.outputs.arm_menu_dismissal();

                Task::none()
            }
            Message::BarReleased => {
                let task = self.outputs.dismiss_armed_menus(&self.config);
                self.attend_the_open_menu();

                task
            }
            Message::CloseMenu(id) => {
                self.wallpaper_pending = None;
                self.bar_layout_pending = None;
                let task = self.outputs.close_menu(id, &self.config);
                self.attend_the_open_menu();

                task
            }
            Message::CloseAllMenus => {
                self.wallpaper_pending = None;
                self.bar_layout_pending = None;
                if self.outputs.menu_is_open() {
                    let task = self.outputs.close_all_menus(&self.config);
                    self.attend_the_open_menu();

                    task
                } else {
                    Task::none()
                }
            }
            _ => Task::none()
        }
    }
}
