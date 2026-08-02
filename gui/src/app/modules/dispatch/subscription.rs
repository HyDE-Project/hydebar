//! Per module dispatch of the subscription of a bar module.

use hydebar_core::config::ModuleName;
use iced::Subscription;
use log::error;

use crate::app::state::{App, Message};

impl App {
    pub(crate) fn get_module_subscription(
        &self,
        module_name: &ModuleName
    ) -> Option<Subscription<Message>> {
        use hydebar_core::modules::Module;

        match module_name {
            ModuleName::AppLauncher => self.app_launcher.subscription(),
            ModuleName::Custom(name) => {
                let Some(module) = self.custom.get(name) else {
                    error!("Custom module `{name}` not found");
                    return None;
                };

                if self
                    .config
                    .custom_modules
                    .iter()
                    .any(|definition| &definition.name == name)
                {
                    module.subscription()
                } else {
                    error!("Custom module def `{name}` not found");
                    None
                }
            }
            ModuleName::Updates => self.updates.subscription(),
            ModuleName::Clipboard => self.clipboard.subscription(),
            ModuleName::Workspaces => self.workspaces.subscription(),
            ModuleName::WindowTitle => self.window_title.subscription(),
            ModuleName::SystemInfo
            | ModuleName::Cpu
            | ModuleName::Memory
            | ModuleName::CpuTemp
            | ModuleName::GpuTemp => self.system_info.subscription(),
            ModuleName::KeyboardLayout => self.keyboard_layout.subscription(),
            ModuleName::KeyboardSubmap => self.keyboard_submap.subscription(),
            ModuleName::Tray => self.tray.subscription(),
            ModuleName::Taskbar => self.taskbar.subscription(),
            ModuleName::Clock
            | ModuleName::HydeMenu
            | ModuleName::Themes
            | ModuleName::Wallpaper
            | ModuleName::BarLayout
            | ModuleName::Battery
            | ModuleName::IdleInhibitor
            | ModuleName::KeybindHint
            | ModuleName::NightLight
            | ModuleName::GameMode
            | ModuleName::Weather
            | ModuleName::Settings => None,
            ModuleName::Privacy => self.privacy.subscription(),
            ModuleName::ControlCenter
            | ModuleName::Audio
            | ModuleName::Network
            | ModuleName::Bluetooth
            | ModuleName::PowerProfile
            | ModuleName::Brightness => self.control_center.subscription(),
            ModuleName::MediaPlayer => self.media_player.subscription(),
            ModuleName::Notifications => self.notifications.subscription(),
            ModuleName::Screenshot => self.screenshot.subscription()
        }
    }
}
