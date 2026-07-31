//! Wiring of modules to the event bus.

use std::collections::HashMap;

use hydebar_core::{
    ModuleContext,
    config::ConfigImpact,
    modules::{self, custom_module::Custom}
};
use hydebar_proto::config::{Config, ModuleName};
use log::error;

use super::super::state::{App, Message};

/// Applies the one law of registration: a module is wired while the layout
/// hosts it and released the moment it is not.
///
/// Every module used to restate this law as its own if/else block, and the
/// copies drifted — one dropped its deregister arm, another swallowed its
/// registration error. Stating the law once means a module cannot forget half
/// of it: a failure is logged under the module's label, and an unhosted module
/// always gives its background work back.
fn gate<M>(
    label: &'static str,
    hosted: bool,
    module: &mut M,
    ctx: &ModuleContext,
    data: M::RegistrationData<'_>
) where
    M: modules::Module<Message>
{
    if hosted {
        if let Err(err) = module.register(ctx, data) {
            error!("failed to register {label} module: {err}");
        }
    } else {
        module.deregister();
    }
}

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

/// Bar entries the system monitor's sampler feeds.
///
/// The standalone processor and memory readouts render from the same sample
/// as the combined monitor, so the sampler has to run while any of the three
/// is on screen.
const SYSTEM_INFO_CONSUMERS: [ModuleName; 5] = [
    ModuleName::SystemInfo,
    ModuleName::Cpu,
    ModuleName::Memory,
    ModuleName::CpuTemp,
    ModuleName::GpuTemp
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
            hosts(ModuleName::Clock) && self.config.clock.show_weather,
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
            "privacy",
            hosts(ModuleName::Privacy),
            &mut self.privacy,
            ctx,
            ()
        );
        gate(
            "settings",
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
            self.config.notifications.source.owns_the_bus(),
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
        gate(
            "idle-inhibitor",
            hosts(ModuleName::IdleInhibitor),
            &mut self.idle_inhibitor,
            ctx,
            ()
        );

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

        let mut schedules: Vec<_> = placed
            .iter()
            .filter_map(|name| {
                self.module_poll_schedule(name)
                    .map(|schedule| (name.clone(), schedule))
            })
            .collect();

        if self.monitor_window_needs_its_own_roster_entry(&placed)
            && let Some(schedule) = self.module_poll_schedule(&ModuleName::SystemInfo)
        {
            schedules.push((ModuleName::SystemInfo, schedule));
        }

        if placed.contains(&ModuleName::CpuTemp)
            && !placed.contains(&ModuleName::Cpu)
            && let Some(schedule) = self.module_poll_schedule(&ModuleName::Cpu)
        {
            schedules.push((ModuleName::Cpu, schedule));
        }

        let roster: Vec<ModuleName> = schedules.iter().map(|(name, _)| name.clone()).collect();

        self.attention.place(schedules);

        if self
            .attention
            .focus()
            .is_some_and(|focus| !placed.contains(focus) && !roster.contains(focus))
        {
            self.attention.look_at(None);
        }
    }

    /// Reports whether the monitor window can open while the monitor itself
    /// is not placed.
    ///
    /// The standalone entries open the monitor's windows, and an open menu
    /// attends its owner rather than the entry it was opened from. Without
    /// the owner on the roster the fast clock would stand still for exactly
    /// the window it exists to keep fresh.
    fn monitor_window_needs_its_own_roster_entry(&self, placed: &[ModuleName]) -> bool {
        !placed.contains(&ModuleName::SystemInfo)
            && SYSTEM_INFO_CONSUMERS
                .iter()
                .skip(1)
                .any(|name| placed.contains(name))
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
