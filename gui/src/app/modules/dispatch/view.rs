//! Per module dispatch of the bar view of a module.

use hydebar_core::{
    config::ModuleName,
    modules::{OnModulePress, hyde_button}
};
use iced::{Element, SurfaceId as Id};
use log::error;

use super::super::actions::custom_module_action;
use crate::{
    app::state::{App, Message},
    views::tray::render_tray
};

impl App {
    #[expect(
        clippy::too_many_lines,
        reason = "one view arm per module name, read as a single dispatch table"
    )]
    pub(crate) fn module_view(
        &self,
        module_name: &ModuleName,
        id: Id,
        opacity: f32
    ) -> Option<(Element<'_, Message>, Option<OnModulePress<Message>>)> {
        match module_name {
            ModuleName::AppLauncher => self
                .modules
                .app_launcher
                .bar_view(&self.config.app_launcher_cmd, self.icons())
                .map(|(content, _)| {
                    (
                        content,
                        Some(OnModulePress::Action(Box::new(Message::OpenLauncher)))
                    )
                }),
            ModuleName::Custom(name) => {
                let Some(definition) = self.config.custom_modules.iter().find(|m| &m.name == name)
                else {
                    error!("Custom module def `{name}` not found");
                    return None;
                };

                let Some(module) = self.modules.custom.get(name) else {
                    error!("Custom module `{name}` not found");
                    return None;
                };

                module
                    .bar_view(definition, self.appearance(), self.icons())
                    .map(|(content, _)| (content, custom_module_action(definition)))
            }
            ModuleName::Updates => self
                .modules
                .updates
                .bar_view(&self.config.updates, self.icons()),
            ModuleName::Clipboard => self
                .modules
                .clipboard
                .bar_view(&self.config.clipboard_cmd, self.icons())
                .map(|(content, _)| {
                    (
                        content,
                        Some(OnModulePress::Action(Box::new(Message::OpenClipboard)))
                    )
                }),
            ModuleName::Workspaces => self.modules.workspaces.bar_view(
                &self.outputs,
                id,
                &self.config.workspaces,
                self.appearance()
            ),
            ModuleName::WindowTitle => self.modules.window_title.bar_view(
                &self.config.window_title,
                self.focus.attention.is_on(module_name)
            ),
            ModuleName::SystemInfo => self.modules.system_info.bar_view(
                &self.config.system,
                self.appearance(),
                self.icons()
            ),
            ModuleName::Cpu => hydebar_core::modules::cpu::bar_view(
                self.modules.system_info.data(),
                &self.config.system,
                self.appearance(),
                self.icons()
            ),
            ModuleName::Memory => hydebar_core::modules::memory::bar_view(
                self.modules.system_info.data(),
                &self.config.system,
                self.modules
                    .system_info
                    .active_memory_format(&self.config.system),
                self.appearance(),
                self.icons()
            ),
            ModuleName::CpuTemp => hydebar_core::modules::cpu_temp::bar_view(
                self.modules.system_info.data(),
                &self.config.system,
                self.appearance(),
                self.icons()
            ),
            ModuleName::GpuTemp => hydebar_core::modules::gpu_temp::bar_view(
                self.modules.system_info.data(),
                &self.config.system,
                self.appearance(),
                self.icons()
            ),
            ModuleName::KeyboardLayout => self
                .modules
                .keyboard_layout
                .bar_view(&self.config.keyboard_layout),
            ModuleName::KeyboardSubmap => self.modules.keyboard_submap.bar_view(),
            ModuleName::Tray => render_tray(&self.modules.tray, id, opacity, self.icons())
                .map(|content| (content, None)),
            ModuleName::Clock => self.modules.clock.bar_view(&self.config.clock),
            ModuleName::HydeMenu => self.modules.hyde_menu.bar_view(self.icons()),
            ModuleName::Battery => self
                .modules
                .battery
                .bar_view(&self.config.battery, self.icons()),
            ModuleName::Privacy => self.modules.privacy.bar_view(self.icons()),
            ModuleName::ControlCenter => self.modules.control_center.bar_view(self.icons()),
            ModuleName::Audio => self.modules.control_center.audio_bar(self.icons()),
            ModuleName::Brightness => self.modules.control_center.brightness_bar(self.icons()),
            ModuleName::Weather => self.modules.weather.bar_view(self.icons()),
            ModuleName::Taskbar => self
                .modules
                .taskbar
                .bar_view(self.config.appearance.font_size_px()),
            ModuleName::KeybindHint => Some(hyde_button::bar_view(
                hyde_button::HydeButton::KeybindHint,
                self.icons(),
                Message::LaunchCommand
            )),
            ModuleName::NightLight => Some(hyde_button::bar_view(
                hyde_button::HydeButton::NightLight,
                self.icons(),
                Message::LaunchCommand
            )),
            ModuleName::GameMode => Some(hyde_button::bar_view(
                hyde_button::HydeButton::GameMode,
                self.icons(),
                Message::LaunchCommand
            )),
            ModuleName::Network => self.modules.control_center.network_bar(self.icons()),
            ModuleName::Bluetooth => self.modules.control_center.bluetooth_bar(self.icons()),
            ModuleName::PowerProfile => {
                self.modules.control_center.power_profile_bar(self.icons())
            }
            ModuleName::Settings => self.modules.settings.bar_view(self.icons()),
            ModuleName::Themes => self.modules.themes.bar_view(self.icons()),
            ModuleName::Wallpaper => self.modules.wallpaper.bar_view(self.icons()),
            ModuleName::BarLayout => self.modules.bar_layout.bar_view(self.icons()),
            ModuleName::MediaPlayer => self
                .modules
                .media_player
                .bar_view(&self.config.media_player, self.icons()),
            ModuleName::Notifications => self.modules.notifications.bar_view(self.icons()),
            ModuleName::Screenshot => self.modules.screenshot.bar_view(self.icons()),
            ModuleName::IdleInhibitor => Some(hydebar_core::modules::idle_inhibitor::bar_view(
                self.modules.control_center.is_idle_inhibited(),
                self.icons()
            ))
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_core::config::CustomModuleDef;
    use iced_test::simulator;

    use super::{
        super::super::super::state::test_support::{test_app, test_app_with},
        *
    };

    fn surface() -> Id {
        Id::unique()
    }

    #[test]
    fn every_module_the_bar_ships_is_dispatched_without_panicking() {
        let app = test_app();

        for module_name in ModuleName::BUILT_IN {
            let _ = app.module_view(module_name, surface(), 1.0);
        }
    }

    #[test]
    fn whatever_a_module_draws_can_be_drawn() {
        let app = test_app();

        for module_name in ModuleName::BUILT_IN {
            if let Some((content, _)) = app.module_view(module_name, surface(), 1.0) {
                let mut ui = simulator(content);

                assert!(
                    ui.snapshot(&iced::Theme::Dark).is_ok(),
                    "{module_name:?} draws"
                );
            }
        }
    }

    #[test]
    fn the_launchers_carry_the_press_that_opens_them() {
        let app = test_app();

        for module_name in [ModuleName::AppLauncher, ModuleName::Clipboard] {
            let (_, press) = app
                .module_view(&module_name, surface(), 1.0)
                .unwrap_or_else(|| panic!("{module_name:?} is drawn"));

            assert!(
                matches!(press, Some(OnModulePress::Action(_))),
                "{module_name:?} answers a press"
            );
        }
    }

    #[test]
    fn a_custom_module_the_configuration_does_not_declare_is_not_drawn() {
        let app = test_app();

        assert!(
            app.module_view(&ModuleName::Custom("gone".to_owned()), surface(), 1.0)
                .is_none()
        );
    }

    #[test]
    fn a_declared_custom_module_is_built_and_drawn() {
        let app = test_app_with(|config| {
            config.custom_modules = vec![CustomModuleDef {
                name: "mine".to_owned(),
                ..CustomModuleDef::default()
            }];
        });

        let (content, _) = app
            .module_view(&ModuleName::Custom("mine".to_owned()), surface(), 1.0)
            .expect("a declared module is built with the bar");

        let mut ui = simulator(content);
        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn a_custom_module_answers_only_to_its_own_name() {
        let app = test_app_with(|config| {
            config.custom_modules = vec![CustomModuleDef {
                name: "mine".to_owned(),
                ..CustomModuleDef::default()
            }];
        });

        assert!(
            app.module_view(&ModuleName::Custom("other".to_owned()), surface(), 1.0)
                .is_none()
        );
    }

    #[test]
    fn the_hyde_buttons_all_launch_a_command() {
        let app = test_app();

        for module_name in [
            ModuleName::KeybindHint,
            ModuleName::NightLight,
            ModuleName::GameMode
        ] {
            let (_, press) = app
                .module_view(&module_name, surface(), 1.0)
                .unwrap_or_else(|| panic!("{module_name:?} is drawn"));

            assert!(press.is_some(), "{module_name:?} answers a press");
        }
    }

    #[test]
    fn the_idle_inhibitor_is_always_drawn() {
        let app = test_app();

        assert!(
            app.module_view(&ModuleName::IdleInhibitor, surface(), 1.0)
                .is_some()
        );
    }
}
