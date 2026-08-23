//! Which module owns a bar entry, stated once for everything asked of it.
//!
//! Several entries are drawn from one module: the standalone processor and
//! memory readouts render from the system monitor's sample, and the audio,
//! network, bluetooth and power entries from the control centre's services.
//! Several others are drawn by a plain function and own no module at all.
//!
//! Subscriptions, sampling cadences and samples are all asked of whatever owns
//! the entry, so the mapping is stated here once and the three questions are
//! answered through it rather than each restating the whole roster.

use hydebar_core::{config::ModuleName, modules::bar::BarModule};
use log::error;

use crate::app::state::{App, Message};

/// The module behind an entry, as something the bar can ask questions of.
type Owner<'a> = &'a dyn BarModule<Message>;

/// The module behind an entry, mutably.
type OwnerMut<'a> = &'a mut dyn BarModule<Message>;

impl App {
    /// The module `module_name` is drawn from, where one owns it.
    ///
    /// Answers [`None`] for an entry drawn by a plain function — the battery,
    /// the temperatures, the desktop's own buttons — which owns no state to
    /// subscribe to or sample.
    pub(crate) fn module_owner(&self, module_name: &ModuleName) -> Option<Owner<'_>> {
        Some(match module_name {
            ModuleName::AppLauncher => &self.app_launcher,
            ModuleName::Clipboard => &self.clipboard,
            ModuleName::Custom(name) => self.declared_custom(name)?,
            ModuleName::Updates => &self.updates,
            ModuleName::Workspaces => &self.workspaces,
            ModuleName::WindowTitle => &self.window_title,
            ModuleName::SystemInfo
            | ModuleName::Cpu
            | ModuleName::Memory
            | ModuleName::CpuTemp
            | ModuleName::GpuTemp => &self.system_info,
            ModuleName::KeyboardLayout => &self.keyboard_layout,
            ModuleName::KeyboardSubmap => &self.keyboard_submap,
            ModuleName::Tray => &self.tray,
            ModuleName::Taskbar => &self.taskbar,
            ModuleName::Clock => &self.clock,
            ModuleName::HydeMenu => &self.hyde_menu,
            ModuleName::Weather => &self.weather,
            ModuleName::Privacy => &self.privacy,
            ModuleName::ControlCenter
            | ModuleName::Audio
            | ModuleName::Network
            | ModuleName::Bluetooth
            | ModuleName::PowerProfile
            | ModuleName::Brightness => &self.control_center,
            ModuleName::MediaPlayer => &self.media_player,
            ModuleName::Notifications => &self.notifications,
            ModuleName::Battery
            | ModuleName::BarLayout
            | ModuleName::Wallpaper
            | ModuleName::Themes
            | ModuleName::Settings
            | ModuleName::Screenshot
            | ModuleName::IdleInhibitor
            | ModuleName::KeybindHint
            | ModuleName::NightLight
            | ModuleName::GameMode => return None
        })
    }

    /// [`module_owner`](App::module_owner), for what has to be changed.
    pub(crate) fn module_owner_mut(&mut self, module_name: &ModuleName) -> Option<OwnerMut<'_>> {
        Some(match module_name {
            ModuleName::AppLauncher => &mut self.app_launcher,
            ModuleName::Clipboard => &mut self.clipboard,
            ModuleName::Custom(name) => {
                let declared = self
                    .config
                    .custom_modules
                    .iter()
                    .any(|definition| &definition.name == name);

                if !declared {
                    return None;
                }

                self.custom.get_mut(name)?
            }
            ModuleName::Updates => &mut self.updates,
            ModuleName::Workspaces => &mut self.workspaces,
            ModuleName::WindowTitle => &mut self.window_title,
            ModuleName::SystemInfo
            | ModuleName::Cpu
            | ModuleName::Memory
            | ModuleName::CpuTemp
            | ModuleName::GpuTemp => &mut self.system_info,
            ModuleName::KeyboardLayout => &mut self.keyboard_layout,
            ModuleName::KeyboardSubmap => &mut self.keyboard_submap,
            ModuleName::Tray => &mut self.tray,
            ModuleName::Taskbar => &mut self.taskbar,
            ModuleName::Clock => &mut self.clock,
            ModuleName::HydeMenu => &mut self.hyde_menu,
            ModuleName::Weather => &mut self.weather,
            ModuleName::Privacy => &mut self.privacy,
            ModuleName::ControlCenter
            | ModuleName::Audio
            | ModuleName::Network
            | ModuleName::Bluetooth
            | ModuleName::PowerProfile
            | ModuleName::Brightness => &mut self.control_center,
            ModuleName::MediaPlayer => &mut self.media_player,
            ModuleName::Notifications => &mut self.notifications,
            ModuleName::Battery
            | ModuleName::BarLayout
            | ModuleName::Wallpaper
            | ModuleName::Themes
            | ModuleName::Settings
            | ModuleName::Screenshot
            | ModuleName::IdleInhibitor
            | ModuleName::KeybindHint
            | ModuleName::NightLight
            | ModuleName::GameMode => return None
        })
    }

    /// The custom module built for `name`, if the configuration declares it.
    ///
    /// A module the bar built and the configuration has since dropped, and a
    /// name the configuration declares and the bar never built, are both a
    /// mistake worth naming rather than a silent nothing.
    fn declared_custom(&self, name: &str) -> Option<Owner<'_>> {
        let Some(module) = self.custom.get(name) else {
            error!("Custom module `{name}` not found");

            return None;
        };

        if !self
            .config
            .custom_modules
            .iter()
            .any(|definition| definition.name == name)
        {
            error!("Custom module def `{name}` not found");

            return None;
        }

        Some(module)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_core::config::{CustomModuleDef, ModuleName};

    use super::super::super::super::state::test_support::{test_app, test_app_with};

    #[test]
    fn every_module_the_bar_ships_is_answered_for_one_way_or_the_other() {
        let mut app = test_app();

        for module_name in ModuleName::BUILT_IN {
            let owned = app.module_owner(&module_name).is_some();
            let owned_mut = app.module_owner_mut(&module_name).is_some();

            assert_eq!(
                owned, owned_mut,
                "{module_name:?} is owned by one lookup and not the other"
            );
        }
    }

    #[test]
    fn an_entry_drawn_by_a_plain_function_owns_nothing() {
        let app = test_app();

        for module_name in [
            ModuleName::Battery,
            ModuleName::IdleInhibitor,
            ModuleName::KeybindHint,
            ModuleName::NightLight,
            ModuleName::GameMode
        ] {
            assert!(
                app.module_owner(&module_name).is_none(),
                "{module_name:?} owns no module"
            );
        }
    }

    #[test]
    fn the_readouts_sharing_a_sample_are_owned_by_the_one_that_takes_it() {
        let app = test_app();

        for module_name in [
            ModuleName::SystemInfo,
            ModuleName::Cpu,
            ModuleName::Memory,
            ModuleName::CpuTemp,
            ModuleName::GpuTemp
        ] {
            assert!(
                app.module_poll_schedule(&module_name).is_some(),
                "{module_name:?} is sampled on the monitor's cadence"
            );
        }
    }

    #[test]
    fn a_custom_module_the_configuration_dropped_owns_nothing() {
        let app = test_app();

        assert!(
            app.module_owner(&ModuleName::Custom("gone".to_owned()))
                .is_none()
        );
    }

    #[test]
    fn a_declared_custom_module_owns_itself() {
        let app = test_app_with(|config| {
            config.custom_modules = vec![CustomModuleDef {
                name: "mine".to_owned(),
                ..CustomModuleDef::default()
            }];
        });

        assert!(
            app.module_owner(&ModuleName::Custom("mine".to_owned()))
                .is_some()
        );
    }
}
