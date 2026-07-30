//! Per module dispatch of the view and subscription of a bar module.

use hydebar_core::{
    attention::PollSchedule, config::ModuleName, menu::MenuType, modules::OnModulePress
};
use iced::{Element, Subscription, SurfaceId as Id};
use log::error;

use super::actions::custom_module_action;
use crate::app::state::{App, Message};

impl App {
    /// The cadences `module_name` declared, if it can be polled at all.
    ///
    /// The five readouts behind the control centre share one set of services,
    /// so both the panel and the standalone network entry answer with the same
    /// schedule: whichever of them the user is looking at, the list of nearby
    /// networks is what wants refreshing.
    pub(crate) fn module_poll_schedule(&self, module_name: &ModuleName) -> Option<PollSchedule> {
        use hydebar_core::modules::Module;

        match module_name {
            ModuleName::ControlCenter | ModuleName::Network => {
                Module::<Message>::poll_schedule(&self.control_center)
            }
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
            _ => Ok(())
        };

        if let Err(err) = outcome {
            error!("failed to poll {module_name:?} module: {err}");
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
            ModuleName::KeyboardLayout => self.keyboard_layout.view(&self.config.keyboard_layout),
            ModuleName::KeyboardSubmap => self.keyboard_submap.view(()),
            ModuleName::Tray => self.tray.view((id, opacity)),
            ModuleName::Clock => self.clock.view(&self.config.clock),
            ModuleName::HydeMenu => self.hyde_menu.view(self.icons()),
            ModuleName::Battery => self.battery.data().map(|data| {
                (
                    crate::views::battery::render_battery(
                        data,
                        &self.config.battery,
                        self.icons()
                    ),
                    self.config
                        .battery
                        .open_settings_on_click
                        .then(|| OnModulePress::ToggleMenu(MenuType::ControlCenter))
                )
            }),
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
            ModuleName::Notifications => self.notifications.view(()),
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
            ModuleName::SystemInfo => self.system_info.subscription(),
            ModuleName::KeyboardLayout => self.keyboard_layout.subscription(),
            ModuleName::KeyboardSubmap => self.keyboard_submap.subscription(),
            ModuleName::Tray => self.tray.subscription(),
            ModuleName::Clock => None,
            ModuleName::HydeMenu => None,
            ModuleName::Themes => None,
            ModuleName::Wallpaper => None,
            ModuleName::Battery => None,
            ModuleName::Privacy => self.privacy.subscription(),
            ModuleName::ControlCenter
            | ModuleName::Audio
            | ModuleName::Network
            | ModuleName::Bluetooth
            | ModuleName::PowerProfile => self.control_center.subscription(),
            ModuleName::MediaPlayer => self.media_player.subscription(),
            ModuleName::Notifications => self.notifications.subscription(),
            ModuleName::Screenshot => self.screenshot.subscription(),
            ModuleName::Settings => None,
            ModuleName::IdleInhibitor => Module::<Message>::subscription(&self.idle_inhibitor)
        }
    }
}
