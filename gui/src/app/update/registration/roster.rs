//! The roster of modules wired to the event bus on every reload.

use hydebar_core::modules;
use hydebar_proto::config::ModuleName;
use log::error;

use super::{
    super::super::state::{App, Message},
    gate::{CONTROL_CENTER_CONSUMERS, SYSTEM_INFO_CONSUMERS, gate, notifications_hosted}
};

impl App {
    /// Registers the background work of every module the layout draws, and
    /// releases it for every module it does not.
    ///
    /// Registration is what starts pollers, D-Bus listeners and scheduled
    /// commands, and none of them has a reader unless the module is rendered
    /// somewhere. Gating on placement is what keeps an idle bar idle: a session
    /// showing only a clock and workspaces pays for a clock and workspaces
    /// rather than for every module the binary happens to ship.
    ///
    /// Called again after every configuration reload, so a module added to or
    /// removed from the layout starts and stops with it.
    #[expect(
        clippy::too_many_lines,
        reason = "one gate call per module, read as a single registration roster"
    )]
    pub(crate) fn register_modules(&mut self) {
        let ctx = &self.module_context;

        let layout = &self.config.modules;
        let hosts = |name: ModuleName| layout.hosts(&name);

        gate(
            "app-launcher",
            hosts(ModuleName::AppLauncher),
            &mut self.app_launcher,
            ctx,
            ()
        );
        gate(
            "clipboard",
            hosts(ModuleName::Clipboard),
            &mut self.clipboard,
            ctx,
            ()
        );
        gate(
            "hyde-menu",
            hosts(ModuleName::HydeMenu),
            &mut self.hyde_menu,
            ctx,
            ()
        );
        gate(
            "clock",
            hosts(ModuleName::Clock),
            &mut self.clock,
            ctx,
            &self.config.clock
        );
        gate(
            "weather",
            hosts(ModuleName::Weather)
                || (hosts(ModuleName::Clock) && self.config.clock.show_weather),
            &mut self.weather,
            ctx,
            &self.config.weather
        );
        gate(
            "updates",
            hosts(ModuleName::Updates),
            &mut self.updates,
            ctx,
            self.config.updates.as_ref()
        );
        gate(
            "workspaces",
            hosts(ModuleName::Workspaces),
            &mut self.workspaces,
            ctx,
            &self.config.workspaces
        );
        gate(
            "window-title",
            hosts(ModuleName::WindowTitle),
            &mut self.window_title,
            ctx,
            ()
        );
        gate(
            "system-info",
            layout.hosts_any(&SYSTEM_INFO_CONSUMERS),
            &mut self.system_info,
            ctx,
            (&self.config.system, layout.hosts(&ModuleName::SystemInfo))
        );
        gate(
            "keyboard-layout",
            hosts(ModuleName::KeyboardLayout),
            &mut self.keyboard_layout,
            ctx,
            ()
        );
        gate(
            "keyboard-submap",
            hosts(ModuleName::KeyboardSubmap),
            &mut self.keyboard_submap,
            ctx,
            ()
        );
        gate("tray", hosts(ModuleName::Tray), &mut self.tray, ctx, ());
        gate(
            "taskbar",
            hosts(ModuleName::Taskbar),
            &mut self.taskbar,
            ctx,
            ()
        );
        gate("desk", self.config.desk.enabled, &mut self.desk, ctx, ());
        gate(
            "privacy",
            hosts(ModuleName::Privacy),
            &mut self.privacy,
            ctx,
            ()
        );
        gate(
            "hardware-services",
            layout.hosts_any(&CONTROL_CENTER_CONSUMERS),
            &mut self.control_center,
            ctx,
            ()
        );
        gate(
            "media-player",
            hosts(ModuleName::MediaPlayer),
            &mut self.media_player,
            ctx,
            ()
        );
        gate(
            "notifications",
            notifications_hosted(&self.config),
            &mut self.notifications,
            ctx,
            ()
        );
        gate(
            "screenshot",
            hosts(ModuleName::Screenshot),
            &mut self.screenshot,
            ctx,
            ()
        );
        for definition in &self.config.custom_modules {
            let placed = layout.hosts(&ModuleName::Custom(definition.name.clone()));

            match self.custom.get_mut(&definition.name) {
                Some(module) => gate(&definition.name, placed, module, ctx, definition),
                None => error!(
                    "custom module '{}' missing runtime state entry during registration",
                    definition.name
                )
            }
        }

        for (name, module) in &mut self.custom {
            if !self
                .config
                .custom_modules
                .iter()
                .any(|definition| definition.name == *name)
            {
                modules::Module::<Message>::deregister(module);
            }
        }

        self.place_pollable_modules();
    }
}
