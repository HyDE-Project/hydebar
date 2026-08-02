//! What each menu prepares for itself on the press that opens it.

use hydebar_core::{
    menu::MenuType,
    modules::{self, control_center::SubMenu},
    position_button::ButtonUIRef
};
use iced::{SurfaceId as Id, Task};

use super::super::super::state::{App, Message};

impl App {
    /// Toggles the menu a bar press asked for, readying its content first.
    pub(super) fn on_toggle_menu(
        &mut self,
        menu_type: MenuType,
        id: Id,
        button_ui_ref: ButtonUIRef
    ) -> Task<Message> {
        self.hints.dismiss();
        self.wallpaper_pending = None;
        self.bar_layout_pending = None;

        let mut cmd = vec![];
        match &menu_type {
            MenuType::Updates => {
                self.updates.collapse();
            }
            MenuType::Tray(name) => {
                if self
                    .tray
                    .service
                    .as_ref()
                    .is_some_and(|t| t.iter().any(|t| &t.name == name))
                {
                    self.tray.collapse_submenus();
                }
            }
            MenuType::Wallpaper => {
                if self.outputs.open_menu() != Some(&MenuType::Wallpaper) {
                    cmd.push(self.wallpaper.load_entries().map(Message::Wallpaper));

                    if self.wallpaper.is_empty() {
                        self.wallpaper_pending = Some((id, button_ui_ref));
                        cmd.push(self.outputs.close_all_menus(&self.config));
                        self.attend_the_open_menu();

                        return Task::batch(cmd);
                    }
                }
            }
            MenuType::BarLayout => {
                if self.outputs.open_menu() != Some(&MenuType::BarLayout) {
                    cmd.push(self.bar_layout.load_entries().map(Message::BarLayout));

                    if self.bar_layout.is_empty() {
                        self.bar_layout_pending = Some((id, button_ui_ref));
                        cmd.push(self.outputs.close_all_menus(&self.config));
                        self.attend_the_open_menu();

                        return Task::batch(cmd);
                    }
                }
            }
            MenuType::Themes => {
                if self.outputs.open_menu() != Some(&MenuType::Themes) {
                    cmd.push(self.themes.load_swatches().map(Message::Themes));
                    cmd.push(self.themes.load_catalogue().map(Message::Themes));
                }
            }
            MenuType::Audio => {
                self.control_center.open_audio_menu();
            }
            MenuType::HydeMenu => {
                cmd.push(self.hyde_menu.reload().map(Message::HydeMenu));
            }
            MenuType::Network => {
                if self.outputs.open_menu() != Some(&MenuType::Network) {
                    self.control_center.close_submenu();
                    self.control_center.update(
                        modules::control_center::Message::ToggleSubMenu(SubMenu::Wifi),
                        &self.config.control_center,
                        &mut self.outputs,
                        &self.config
                    );
                }
            }
            MenuType::Bluetooth => {
                self.control_center.open_bluetooth_menu();
            }
            MenuType::ControlCenter => {
                self.control_center.close_submenu();
                cmd.push(
                    self.control_center
                        .refresh_brightness()
                        .map(Message::ControlCenter)
                );
            }
            _ => {}
        }
        cmd.push(
            self.outputs
                .toggle_menu(id, menu_type, button_ui_ref, &self.config)
        );

        self.attend_the_open_menu();

        Task::batch(cmd)
    }
}
