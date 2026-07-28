//! Per module dispatch of the view and subscription of a bar module.

use hydebar_core::{config::ModuleName, menu::MenuType, modules::OnModulePress};
use iced::{Element, Subscription, window::Id};
use log::error;

use super::actions::custom_module_action;
use crate::app::state::{App, Message};

impl App {
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
            ModuleName::WindowTitle => self.window_title.view(()),
            ModuleName::SystemInfo => {
                self.system_info
                    .view((&self.config.system, self.appearance(), self.icons()))
            }
            ModuleName::KeyboardLayout => self.keyboard_layout.view(&self.config.keyboard_layout),
            ModuleName::KeyboardSubmap => self.keyboard_submap.view(()),
            ModuleName::Tray => self.tray.view((id, opacity)),
            ModuleName::Clock => self.clock.view(&self.config.clock),
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
                        .then(|| OnModulePress::ToggleMenu(MenuType::Settings))
                )
            }),
            ModuleName::Privacy => self.privacy.view(self.icons()),
            ModuleName::Settings => self.settings.view(self.icons()),
            ModuleName::MediaPlayer => self
                .media_player
                .view((&self.config.media_player, self.icons())),
            ModuleName::Notifications => self.notifications.view(()),
            ModuleName::Screenshot => self.screenshot.view(self.icons()),
            ModuleName::IdleInhibitor => self
                .idle_inhibitor
                .view((self.settings.is_idle_inhibited(), self.icons()))
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
            ModuleName::Battery => None,
            ModuleName::Privacy => self.privacy.subscription(),
            ModuleName::Settings => self.settings.subscription(),
            ModuleName::MediaPlayer => self.media_player.subscription(),
            ModuleName::Notifications => self.notifications.subscription(),
            ModuleName::Screenshot => self.screenshot.subscription(),
            ModuleName::IdleInhibitor => Module::<Message>::subscription(&self.idle_inhibitor)
        }
    }
}
