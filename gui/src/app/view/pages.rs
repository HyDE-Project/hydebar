//! The table naming what every menu window shows.

use hydebar_core::{
    menu::{MenuSize, MenuType},
    modules::{
        control_center::{ControlCenterViewExt, audio::AudioMessage},
        custom_module
    }
};
use iced::{Element, SurfaceId as Id};

use super::super::{
    modules::actions::custom_menu_message,
    state::{App, Message}
};

impl App {
    /// Content, width and measured height of the window `menu_type` opens.
    ///
    /// The one table naming what every menu shows: the wrapping, placement and
    /// fade around it are the same for all of them and live with the caller.
    /// [`None`] stands for a menu whose owner is gone, such as a custom module
    /// the configuration no longer declares.
    #[allow(clippy::type_complexity)]
    #[expect(
        clippy::too_many_lines,
        reason = "one table naming what every menu shows, one arm per menu"
    )]
    pub(super) fn menu_page(
        &self,
        menu_type: &MenuType,
        id: Id,
        opacity: f32
    ) -> Option<(Element<'_, Message>, MenuSize, Option<f32>)> {
        match menu_type {
            MenuType::Updates => Some((
                self.updates
                    .menu_view(id, opacity, self.icons())
                    .map(Message::Updates),
                MenuSize::Small,
                None
            )),
            MenuType::Tray(name) => Some((
                self.tray
                    .menu_view(name, opacity, self.icons())
                    .map(Message::Tray),
                MenuSize::Small,
                None
            )),
            MenuType::ControlCenter => Some((
                self.control_center
                    .menu_view(
                        id,
                        &self.config.control_center,
                        opacity,
                        self.config.position,
                        self.icons()
                    )
                    .map(Message::ControlCenter),
                MenuSize::Medium,
                None
            )),
            MenuType::Audio => Some((
                iced::widget::mouse_area(
                    self.control_center
                        .audio_menu(
                            id,
                            &self.config.control_center,
                            opacity,
                            self.config.position,
                            self.icons()
                        )
                        .map(Message::ControlCenter)
                )
                .on_scroll(|delta| {
                    Message::ControlCenter(hydebar_core::modules::control_center::Message::Audio(
                        AudioMessage::SinkVolumeWheel(
                            hydebar_core::modules::control_center::audio::wheel_direction(delta)
                        )
                    ))
                })
                .into(),
                MenuSize::Medium,
                None
            )),
            MenuType::Network => Some((
                self.control_center
                    .network_menu(id, &self.config.control_center, opacity, self.icons())
                    .map(Message::ControlCenter),
                MenuSize::Medium,
                None
            )),
            MenuType::Bluetooth => Some((
                self.control_center
                    .bluetooth_menu(id, &self.config.control_center, opacity, self.icons())
                    .map(Message::ControlCenter),
                MenuSize::Medium,
                None
            )),
            MenuType::PowerProfile => Some((
                self.control_center
                    .power_profile_menu(opacity, &self.config.control_center, self.icons())
                    .map(Message::ControlCenter),
                MenuSize::Small,
                None
            )),
            MenuType::HydeMenu => Some((
                self.hyde_menu.menu_view(id, opacity).map(Message::HydeMenu),
                MenuSize::Small,
                None
            )),
            MenuType::MediaPlayer => Some((
                self.media_player
                    .menu_view(&self.config.media_player, opacity, self.icons())
                    .map(Message::MediaPlayer),
                MenuSize::Large,
                None
            )),
            MenuType::Wallpaper => Some((
                self.wallpaper
                    .menu_view(self.appearance().font_size_px())
                    .map(Message::Wallpaper),
                MenuSize::Medium,
                None
            )),
            MenuType::BarLayout => Some((
                self.bar_layout
                    .menu_view(self.appearance().font_size_px())
                    .map(Message::BarLayout),
                MenuSize::Small,
                None
            )),
            MenuType::Notifications => Some((
                self.notifications
                    .menu_view(opacity, self.icons())
                    .map(Message::Notifications),
                MenuSize::Medium,
                None
            )),
            MenuType::Screenshot => Some((
                self.screenshot
                    .menu_view(opacity, self.icons())
                    .map(Message::Screenshot),
                MenuSize::Small,
                None
            )),
            MenuType::Settings
            | MenuType::Themes
            | MenuType::SystemInfo
            | MenuType::Cpu
            | MenuType::Memory
            | MenuType::CpuTemp
            | MenuType::Gpu
            | MenuType::Calendar => self.measured_page(menu_type, opacity),
            MenuType::Custom(name) => self
                .config
                .custom_modules
                .iter()
                .find(|definition| &definition.name == name)
                .map(|definition| {
                    (
                        custom_module::menu_view(definition, self.appearance(), opacity, {
                            move |entry| custom_menu_message(id, entry)
                        }),
                        MenuSize::Small,
                        None
                    )
                })
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
