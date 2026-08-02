//! Placement of the pollable modules on the two clocks' rosters.

use std::collections::HashMap;

use hydebar_core::{config::ConfigImpact, modules::custom_module::Custom};
use hydebar_proto::config::{Config, ModuleName};

use super::{super::super::state::App, gate::SYSTEM_INFO_CONSUMERS};

impl App {
    /// Hands the two clocks the pollable modules the layout draws.
    ///
    /// Placement is the whole gate: a module the layout does not draw is not on
    /// the roster and is never sampled, so a readout nobody can see costs
    /// nothing. An attention resting on a module the reload dropped is released
    /// for the same reason.
    pub(super) fn place_pollable_modules(&mut self) {
        let placed: Vec<ModuleName> = self.config.modules.placed().cloned().collect();

        let mut schedules: Vec<_> = placed
            .iter()
            .filter_map(|name| {
                self.module_poll_schedule(name)
                    .map(|schedule| (name.clone(), schedule))
            })
            .collect();

        if Self::monitor_window_needs_its_own_roster_entry(&placed)
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
    fn monitor_window_needs_its_own_roster_entry(placed: &[ModuleName]) -> bool {
        !placed.contains(&ModuleName::SystemInfo)
            && SYSTEM_INFO_CONSUMERS
                .iter()
                .skip(1)
                .any(|name| placed.contains(name))
    }

    pub(crate) fn update_custom_modules(&mut self, config: &Config, impact: &ConfigImpact) {
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
