//! Per module dispatch of the bar view of a module.

use hydebar_core::{
    config::ModuleName,
    modules::{OnModulePress, hyde_button}
};
use iced::{Element, SurfaceId as Id};
use log::error;

use super::super::actions::custom_module_action;
use crate::app::state::{App, Message};

impl App {
    #[expect(
        clippy::too_many_lines,
        reason = "one view arm per module name, read as a single dispatch table"
    )]
    pub(crate) fn get_module_view(
        &self,
        module_name: &ModuleName,
        id: Id,
        opacity: f32
    ) -> Option<(Element<'_, Message>, Option<OnModulePress<Message>>)> {
        use hydebar_core::modules::Module;

        match module_name {
            ModuleName::AppLauncher => self
                .app_launcher
                .view((&self.config.app_launcher_cmd, self.icons()))
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

                let Some(module) = self.custom.get(name) else {
                    error!("Custom module `{name}` not found");
                    return None;
                };

                module
                    .view((definition, self.appearance(), self.icons()))
                    .map(|(content, _)| (content, custom_module_action(definition)))
            }
            ModuleName::Updates => self.updates.view((&self.config.updates, self.icons())),
            ModuleName::Clipboard => self
                .clipboard
                .view((&self.config.clipboard_cmd, self.icons()))
                .map(|(content, _)| {
                    (
                        content,
                        Some(OnModulePress::Action(Box::new(Message::OpenClipboard)))
                    )
                }),
            ModuleName::Workspaces => self.workspaces.view((
                &self.outputs,
                id,
                &self.config.workspaces,
                self.appearance()
            )),
            ModuleName::WindowTitle => self
                .window_title
                .view((&self.config.window_title, self.attention.is_on(module_name))),
            ModuleName::SystemInfo => {
                self.system_info
                    .view((&self.config.system, self.appearance(), self.icons()))
            }
            ModuleName::Cpu => hydebar_core::modules::cpu::bar_view(
                self.system_info.data(),
                &self.config.system,
                self.appearance(),
                self.icons()
            ),
            ModuleName::Memory => hydebar_core::modules::memory::bar_view(
                self.system_info.data(),
                &self.config.system,
                self.system_info.active_memory_format(&self.config.system),
                self.appearance(),
                self.icons()
            ),
            ModuleName::CpuTemp => hydebar_core::modules::cpu_temp::bar_view(
                self.system_info.data(),
                &self.config.system,
                self.appearance(),
                self.icons()
            ),
            ModuleName::GpuTemp => hydebar_core::modules::gpu_temp::bar_view(
                self.system_info.data(),
                &self.config.system,
                self.appearance(),
                self.icons()
            ),
            ModuleName::KeyboardLayout => self.keyboard_layout.view(&self.config.keyboard_layout),
            ModuleName::KeyboardSubmap => self.keyboard_submap.view(()),
            ModuleName::Tray => {
                crate::views::tray::render_tray(&self.tray, id, opacity, self.icons())
                    .map(|content| (content, None))
            }
            ModuleName::Clock => self.clock.view(&self.config.clock),
            ModuleName::HydeMenu => self.hyde_menu.view(self.icons()),
            ModuleName::Battery => self.battery.bar_view(&self.config.battery, self.icons()),
            ModuleName::Privacy => self.privacy.view(self.icons()),
            ModuleName::ControlCenter => self.control_center.view(self.icons()),
            ModuleName::Audio => self.control_center.audio_bar(self.icons()),
            ModuleName::Brightness => self.control_center.brightness_bar(self.icons()),
            ModuleName::Weather => self.weather.bar_view(self.icons()),
            ModuleName::Taskbar => self.taskbar.view(self.config.appearance.font_size_px()),
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
            ModuleName::Network => self.control_center.network_bar(self.icons()),
            ModuleName::Bluetooth => self.control_center.bluetooth_bar(self.icons()),
            ModuleName::PowerProfile => self.control_center.power_profile_bar(self.icons()),
            ModuleName::Settings => self.settings.view(self.icons()),
            ModuleName::Themes => self.themes.view(self.icons()),
            ModuleName::Wallpaper => self.wallpaper.view(self.icons()),
            ModuleName::BarLayout => self.bar_layout.view(self.icons()),
            ModuleName::MediaPlayer => self
                .media_player
                .view((&self.config.media_player, self.icons())),
            ModuleName::Notifications => self.notifications.view(self.icons()),
            ModuleName::Screenshot => self.screenshot.view(self.icons()),
            ModuleName::IdleInhibitor => Some(hydebar_core::modules::idle_inhibitor::bar_view(
                self.control_center.is_idle_inhibited(),
                self.icons()
            ))
        }
    }
}
