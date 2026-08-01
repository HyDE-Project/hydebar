//! Per module dispatch of the view and subscription of a bar module.

use hydebar_core::{attention::PollSchedule, config::ModuleName, modules::OnModulePress};
use iced::{Element, Subscription, SurfaceId as Id};
use log::error;

use super::actions::custom_module_action;
use crate::app::state::{App, Message};

impl App {
    /// The cadences `module_name` declared, if it can be polled at all.
    ///
    /// Thin bar entries answer with the schedule of the module that owns
    /// their data: the standalone network entry with the control centre's,
    /// the processor and memory entries with the system monitor's. Whichever
    /// of them the user is looking at, the shared readouts are what want
    /// refreshing.
    pub(crate) fn module_poll_schedule(&self, module_name: &ModuleName) -> Option<PollSchedule> {
        use hydebar_core::modules::Module;

        match module_name {
            ModuleName::ControlCenter | ModuleName::Network => {
                Module::<Message>::poll_schedule(&self.control_center)
            }
            ModuleName::SystemInfo
            | ModuleName::Cpu
            | ModuleName::Memory
            | ModuleName::CpuTemp
            | ModuleName::GpuTemp => Module::<Message>::poll_schedule(&self.system_info),
            _ => None
        }
    }

    /// Takes one sample of `module_name`.
    pub(crate) fn poll_module(&mut self, module_name: &ModuleName) {
        use hydebar_core::modules::Module;

        let ctx = self.module_context.clone();
        let outcome = match module_name {
            ModuleName::ControlCenter | ModuleName::Network => {
                Module::<Message>::poll(&mut self.control_center, &ctx)
            }
            ModuleName::SystemInfo
            | ModuleName::Cpu
            | ModuleName::Memory
            | ModuleName::CpuTemp
            | ModuleName::GpuTemp => Module::<Message>::poll(&mut self.system_info, &ctx),
            _ => Ok(())
        };

        if let Err(err) = outcome {
            error!("failed to poll {module_name:?} module: {err}");
        }
    }

    /// Samples the attended module at once, when its cadence owes one.
    ///
    /// Attention lands on a hover or on an opened menu, but the fast clock
    /// only fires a whole period later — and a window opened onto readouts
    /// sampled seconds ago reads as wrong until it does. The cadence still
    /// gates the call, so resting the pointer on a module cannot sample it
    /// faster than it declared.
    pub(crate) fn poll_attended_now(&mut self) {
        let now = std::time::Instant::now();

        if let Some(module) = self.attention.due_attended(now) {
            self.poll_module(&module);
        }
    }

    pub(super) fn get_module_view(
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
            ModuleName::Network => self.control_center.network_bar(self.icons()),
            ModuleName::Bluetooth => self.control_center.bluetooth_bar(self.icons()),
            ModuleName::PowerProfile => self.control_center.power_profile_bar(self.icons()),
            ModuleName::Settings => self.settings.view(self.icons()),
            ModuleName::Themes => self.themes.view(self.icons()),
            ModuleName::Wallpaper => self.wallpaper.view(self.icons()),
            ModuleName::MediaPlayer => self
                .media_player
                .view((&self.config.media_player, self.icons())),
            ModuleName::Notifications => self.notifications.view(self.icons()),
            ModuleName::Screenshot => self.screenshot.view(self.icons()),
            ModuleName::IdleInhibitor => self
                .idle_inhibitor
                .view((self.control_center.is_idle_inhibited(), self.icons()))
        }
    }

    pub(super) fn get_module_subscription(
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
            ModuleName::Clock
            | ModuleName::HydeMenu
            | ModuleName::Themes
            | ModuleName::Wallpaper
            | ModuleName::Battery
            | ModuleName::Settings => None,
            ModuleName::Privacy => self.privacy.subscription(),
            ModuleName::ControlCenter
            | ModuleName::Audio
            | ModuleName::Network
            | ModuleName::Bluetooth
            | ModuleName::PowerProfile => self.control_center.subscription(),
            ModuleName::MediaPlayer => self.media_player.subscription(),
            ModuleName::Notifications => self.notifications.subscription(),
            ModuleName::Screenshot => self.screenshot.subscription(),
            ModuleName::IdleInhibitor => Module::<Message>::subscription(&self.idle_inhibitor)
        }
    }
}
