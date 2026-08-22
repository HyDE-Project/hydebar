//! The table naming what every menu window shows.
//!
//! One table, kept in one place: the wrapping, placement and fade around a
//! menu are the same for all of them and live with the caller. What each of
//! them shows is grouped by whose menu it is — [`control_centre`] for the
//! machine's own switches, [`notices`] for what the session has to say and
//! [`desktop`] for the desk's own windows — so a menu added to one owner
//! leaves the others untouched.

mod control_centre;
mod desktop;
mod notices;

use hydebar_core::menu::{MenuSize, MenuType};
use iced::{Element, SurfaceId as Id};

use super::super::state::{App, Message};

/// What one menu window shows: its content, its width and its measured height.
pub(crate) type Page<'a> = (Element<'a, Message>, MenuSize, Option<f32>);

impl App {
    /// Content, width and measured height of the window `menu_type` opens.
    ///
    /// [`None`] stands for a menu whose owner is gone, such as a custom module
    /// the configuration no longer declares.
    pub(crate) fn menu_page(
        &self,
        menu_type: &MenuType,
        id: Id,
        opacity: f32
    ) -> Option<Page<'_>> {
        match menu_type {
            MenuType::ControlCenter
            | MenuType::Audio
            | MenuType::Network
            | MenuType::Bluetooth
            | MenuType::PowerProfile => self.control_centre_page(menu_type, id, opacity),
            MenuType::Updates
            | MenuType::Tray(_)
            | MenuType::Notifications
            | MenuType::Screenshot
            | MenuType::MediaPlayer => self.notice_page(menu_type, id, opacity),
            MenuType::HydeMenu | MenuType::Wallpaper | MenuType::BarLayout => {
                self.desktop_page(menu_type, id, opacity)
            }
            MenuType::Settings
            | MenuType::Themes
            | MenuType::SystemInfo
            | MenuType::Cpu
            | MenuType::Memory
            | MenuType::CpuTemp
            | MenuType::Gpu
            | MenuType::Calendar => self.measured_page(menu_type, opacity),
            MenuType::Custom(name) => self.custom_page(name, id, opacity)
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use iced_test::simulator;

    use super::{
        super::super::state::test_support::{test_app, test_app_with},
        *
    };

    fn surface() -> Id {
        Id::unique()
    }

    /// Every menu whose page is named outright, with the width it asks for.
    fn plain_menus() -> Vec<(MenuType, MenuSize)> {
        vec![
            (MenuType::Updates, MenuSize::Small),
            (MenuType::Tray("app".to_owned()), MenuSize::Small),
            (MenuType::ControlCenter, MenuSize::Medium),
            (MenuType::Audio, MenuSize::Medium),
            (MenuType::Network, MenuSize::Medium),
            (MenuType::Bluetooth, MenuSize::Medium),
            (MenuType::PowerProfile, MenuSize::Small),
            (MenuType::HydeMenu, MenuSize::Small),
            (MenuType::MediaPlayer, MenuSize::Large),
            (MenuType::Wallpaper, MenuSize::Medium),
            (MenuType::BarLayout, MenuSize::Small),
            (MenuType::Notifications, MenuSize::Medium),
            (MenuType::Screenshot, MenuSize::Small),
        ]
    }

    #[test]
    fn every_plain_menu_names_a_page_of_the_width_it_asks_for() {
        let app = test_app();

        for (menu_type, expected) in plain_menus() {
            let (_, size, measured) = app
                .menu_page(&menu_type, surface(), 1.0)
                .unwrap_or_else(|| panic!("{menu_type:?} names a page"));

            assert_eq!(size, expected, "{menu_type:?} asks for its own width");
            assert!(
                measured.is_none(),
                "{menu_type:?} states no height of its own"
            );
        }
    }

    #[test]
    fn every_plain_menu_draws_what_it_names() {
        let app = test_app();

        for (menu_type, _) in plain_menus() {
            let (content, _, _) = app
                .menu_page(&menu_type, surface(), 1.0)
                .unwrap_or_else(|| panic!("{menu_type:?} names a page"));

            let mut ui = simulator(content);
            assert!(
                ui.snapshot(&iced::Theme::Dark).is_ok(),
                "{menu_type:?} draws"
            );
        }
    }

    #[test]
    fn the_measured_menus_state_a_height_of_their_own() {
        let app = test_app();

        for menu_type in [
            MenuType::Settings,
            MenuType::Themes,
            MenuType::SystemInfo,
            MenuType::Cpu,
            MenuType::Memory,
            MenuType::CpuTemp,
            MenuType::Gpu,
            MenuType::Calendar
        ] {
            let page = app.menu_page(&menu_type, surface(), 1.0);

            assert!(page.is_some(), "{menu_type:?} names a page");
        }
    }

    #[test]
    fn a_custom_menu_the_configuration_no_longer_declares_names_no_page() {
        let app = test_app();

        assert!(
            app.menu_page(&MenuType::Custom("gone".to_owned()), surface(), 1.0)
                .is_none()
        );
    }

    #[test]
    fn a_declared_custom_menu_names_its_own_page() {
        let app = test_app_with(|config| {
            config.custom_modules = vec![hydebar_core::config::CustomModuleDef {
                name: "mine".to_owned(),
                ..hydebar_core::config::CustomModuleDef::default()
            }];
        });

        let (content, size, measured) = app
            .menu_page(&MenuType::Custom("mine".to_owned()), surface(), 1.0)
            .expect("the declared module names a page");

        assert_eq!(size, MenuSize::Small);
        assert!(measured.is_none());

        let mut ui = simulator(content);
        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn a_custom_menu_answers_only_to_its_own_name() {
        let app = test_app_with(|config| {
            config.custom_modules = vec![hydebar_core::config::CustomModuleDef {
                name: "mine".to_owned(),
                ..hydebar_core::config::CustomModuleDef::default()
            }];
        });

        assert!(
            app.menu_page(&MenuType::Custom("other".to_owned()), surface(), 1.0)
                .is_none()
        );
    }
}
