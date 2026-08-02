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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use super::{super::super::super::state::test_support::test_app, *};

    fn surface() -> Id {
        Id::unique()
    }

    fn press() -> ButtonUIRef {
        ButtonUIRef {
            position: iced::Point::new(10.0, 4.0),
            viewport: (1920.0, 34.0)
        }
    }

    /// Every menu a press can open, one of each kind the toggle answers to.
    fn every_menu() -> Vec<MenuType> {
        vec![
            MenuType::Updates,
            MenuType::Tray("app".to_owned()),
            MenuType::Wallpaper,
            MenuType::BarLayout,
            MenuType::Themes,
            MenuType::Audio,
            MenuType::HydeMenu,
            MenuType::Network,
            MenuType::Bluetooth,
            MenuType::ControlCenter,
            MenuType::Settings,
            MenuType::Calendar,
            MenuType::Custom("mine".to_owned()),
        ]
    }

    #[test]
    fn every_menu_readies_itself_for_the_press_that_opens_it() {
        for menu_type in every_menu() {
            let mut app = test_app();

            let _ = app.on_toggle_menu(menu_type.clone(), surface(), press());
        }
    }

    #[test]
    fn a_wallpaper_menu_with_nothing_to_show_waits_for_its_entries() {
        let mut app = test_app();

        let _ = app.on_toggle_menu(MenuType::Wallpaper, surface(), press());

        assert!(
            app.wallpaper_pending.is_some(),
            "the press is held until the wallpapers are read"
        );
    }

    #[test]
    fn a_bar_layout_menu_with_nothing_to_show_waits_for_its_entries() {
        let mut app = test_app();

        let _ = app.on_toggle_menu(MenuType::BarLayout, surface(), press());

        assert!(
            app.bar_layout_pending.is_some(),
            "the press is held until the layouts are read"
        );
    }

    #[test]
    fn a_held_press_is_dropped_when_another_menu_is_asked_for() {
        let mut app = test_app();

        let _ = app.on_toggle_menu(MenuType::Wallpaper, surface(), press());
        assert!(app.wallpaper_pending.is_some());

        let _ = app.on_toggle_menu(MenuType::Updates, surface(), press());

        assert!(app.wallpaper_pending.is_none());
        assert!(app.bar_layout_pending.is_none());
    }

    #[test]
    fn a_tray_menu_of_an_item_the_bar_never_saw_readies_nothing() {
        let mut app = test_app();

        let _ = app.on_toggle_menu(MenuType::Tray("gone".to_owned()), surface(), press());

        assert!(app.tray.service.is_none());
    }
}
