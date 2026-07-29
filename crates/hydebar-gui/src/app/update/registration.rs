//! Wiring of modules to the event bus.

use std::collections::HashMap;

use hydebar_core::{
    config::ConfigImpact,
    modules::{self, custom_module::Custom}
};
use hydebar_proto::config::{Config, ModuleName};
use log::error;

use super::super::state::{App, Message};

/// Bar entries the control centre services feed.
///
/// The five hardware listeners behind the control centre are shared: the
/// standalone `Audio`, `Network`, `Bluetooth` and `PowerProfile` readouts
/// render from the same state as the full panel, and the battery indicator
/// opens its menu. One of them on the bar is enough to justify the connections.
const CONTROL_CENTER_CONSUMERS: [ModuleName; 7] = [
    ModuleName::ControlCenter,
    ModuleName::Audio,
    ModuleName::Network,
    ModuleName::Bluetooth,
    ModuleName::PowerProfile,
    ModuleName::Settings,
    ModuleName::Battery
];

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
    pub(crate) fn register_modules(&mut self) {
        let ctx = &self.module_context;
        let register = |name: &str, result: Result<(), modules::ModuleError>| {
            if let Err(err) = result {
                error!("failed to register {name} module: {err}");
            }
        };

        let layout = self.config.modules.clone();
        let hosts = |name: ModuleName| layout.hosts(&name);

        if hosts(ModuleName::AppLauncher) {
            register(
                "app-launcher",
                modules::Module::<Message>::register(&mut self.app_launcher, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.app_launcher);
        }

        if hosts(ModuleName::Clipboard) {
            register(
                "clipboard",
                modules::Module::<Message>::register(&mut self.clipboard, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.clipboard);
        }

        if hosts(ModuleName::Clock) {
            self.clock.register(ctx, &self.config.clock);
        } else {
            self.clock.stop();
        }

        if hosts(ModuleName::Clock) && self.config.clock.show_weather {
            self.weather.register(ctx);
        } else {
            self.weather.stop();
        }

        if hosts(ModuleName::Updates) {
            register(
                "updates",
                modules::Module::<Message>::register(
                    &mut self.updates,
                    ctx,
                    self.config.updates.as_ref()
                )
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.updates);
        }

        if hosts(ModuleName::Workspaces) {
            register(
                "workspaces",
                modules::Module::<Message>::register(
                    &mut self.workspaces,
                    ctx,
                    &self.config.workspaces
                )
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.workspaces);
        }

        if hosts(ModuleName::WindowTitle) {
            register(
                "window-title",
                modules::Module::<Message>::register(&mut self.window_title, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.window_title);
        }

        if hosts(ModuleName::SystemInfo) {
            register(
                "system-info",
                modules::Module::<Message>::register(
                    &mut self.system_info,
                    ctx,
                    &self.config.system
                )
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.system_info);
        }

        if hosts(ModuleName::KeyboardLayout) {
            register(
                "keyboard-layout",
                modules::Module::<Message>::register(&mut self.keyboard_layout, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.keyboard_layout);
        }

        if hosts(ModuleName::KeyboardSubmap) {
            register(
                "keyboard-submap",
                modules::Module::<Message>::register(&mut self.keyboard_submap, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.keyboard_submap);
        }

        if hosts(ModuleName::Tray) {
            register(
                "tray",
                modules::Module::<Message>::register(&mut self.tray, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.tray);
        }

        self.battery.register(ctx);

        if hosts(ModuleName::Privacy) {
            register(
                "privacy",
                modules::Module::<Message>::register(&mut self.privacy, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.privacy);
        }

        if layout.hosts_any(&CONTROL_CENTER_CONSUMERS) {
            register(
                "settings",
                modules::Module::<Message>::register(&mut self.control_center, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.control_center);
        }

        if hosts(ModuleName::MediaPlayer) {
            register(
                "media-player",
                modules::Module::<Message>::register(&mut self.media_player, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.media_player);
        }

        if self.config.notifications.source.owns_the_bus() {
            register(
                "notifications",
                modules::Module::<Message>::register(&mut self.notifications, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.notifications);
        }

        if hosts(ModuleName::Screenshot) {
            register(
                "screenshot",
                modules::Module::<Message>::register(&mut self.screenshot, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.screenshot);
        }

        if hosts(ModuleName::IdleInhibitor) {
            register(
                "idle-inhibitor",
                modules::Module::<Message>::register(&mut self.idle_inhibitor, ctx, ())
            );
        } else {
            modules::Module::<Message>::deregister(&mut self.idle_inhibitor);
        }

        for definition in &self.config.custom_modules {
            let placed = layout.hosts(&ModuleName::Custom(definition.name.clone()));

            match self.custom.get_mut(&definition.name) {
                Some(module) => {
                    let data = placed.then_some(definition);

                    if let Err(err) = modules::Module::<Message>::register(module, ctx, data) {
                        error!(
                            "failed to register custom module '{}': {err}",
                            definition.name
                        );
                    }
                }
                None => error!(
                    "custom module '{}' missing runtime state entry during registration",
                    definition.name
                )
            }
        }

        for (name, module) in self.custom.iter_mut() {
            if !self
                .config
                .custom_modules
                .iter()
                .any(|definition| definition.name == *name)
                && let Err(err) = modules::Module::<Message>::register(module, ctx, None)
            {
                error!("failed to clear registration for custom module '{name}': {err}");
            }
        }

        self.place_pollable_modules();
    }

    /// Hands the two clocks the pollable modules the layout draws.
    ///
    /// Placement is the whole gate: a module the layout does not draw is not on
    /// the roster and is never sampled, so a readout nobody can see costs
    /// nothing. An attention resting on a module the reload dropped is released
    /// for the same reason.
    fn place_pollable_modules(&mut self) {
        let placed: Vec<ModuleName> = self.config.modules.placed().cloned().collect();

        let schedules: Vec<_> = placed
            .iter()
            .filter_map(|name| {
                self.module_poll_schedule(name)
                    .map(|schedule| (name.clone(), schedule))
            })
            .collect();

        self.attention.place(schedules);

        if self
            .attention
            .focus()
            .is_some_and(|focus| !placed.contains(focus))
        {
            self.attention.look_at(None);
        }
    }

    pub(super) fn update_custom_modules(&mut self, config: &Config, impact: &ConfigImpact) {
        let mut state = HashMap::with_capacity(config.custom_modules.len());

        for module in &config.custom_modules {
            let module_name = module.name.clone();
            let module_key = ModuleName::Custom(module_name.clone());

            let entry = if impact.affects_module(&module_key) {
                Custom::default()
            } else {
                self.custom.remove(module_name.as_str()).unwrap_or_default()
            };

            state.insert(module_name, entry);
        }

        self.custom = state;
    }
}
