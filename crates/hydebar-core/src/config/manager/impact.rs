//! What a configuration reload means for the running bar.
//!
//! Two configurations are compared field by field and the difference is
//! restated as an [`ConfigImpact`]: which modules moved, whether the layout,
//! appearance or position changed, and — through
//! [`ConfigImpact::moves_module_registration`] — whether any background work
//! has to be torn down and restarted at all.

use std::collections::{BTreeSet, HashMap};

use hydebar_proto::config::{Config, CustomModuleDef, ModuleName};

/// Represents the effect a configuration update has on the running system.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag mirrors one independent facet of a reload"
)]
pub struct ConfigImpact {
    /// Modules whose configuration changed and may require additional handling.
    pub affected_modules:       BTreeSet<ModuleName>,
    /// Whether the module layout changed.
    pub layout_changed:         bool,
    /// Whether appearance settings changed.
    pub appearance_changed:     bool,
    /// Whether output targeting changed.
    pub outputs_changed:        bool,
    /// Whether the bar position changed.
    pub position_changed:       bool,
    /// Whether the log level changed.
    pub log_level_changed:      bool,
    /// Whether menu keyboard focus changed.
    pub menu_focus_changed:     bool,
    /// Whether custom module definitions changed.
    pub custom_modules_changed: bool
}

impl ConfigImpact {
    /// Returns `true` if the given module is listed as affected by the update.
    #[must_use]
    pub fn affects_module(&self, module: &ModuleName) -> bool {
        self.affected_modules.contains(module)
    }

    /// Whether the update moves any module's background work.
    ///
    /// Registration starts and stops pollers, D-Bus listeners and spawned
    /// commands, and tears the running ones down first. Only a layout change,
    /// a custom-module change or a module whose own configuration moved can
    /// call for that; a reload that merely recolours the bar — which is what
    /// every desktop theme switch amounts to — must leave the listeners alone.
    #[must_use]
    pub fn moves_module_registration(&self) -> bool {
        self.layout_changed || self.custom_modules_changed || !self.affected_modules.is_empty()
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one flat comparison per module keeps every impact visible in one place"
)]
pub(super) fn compute_impact(previous: &Config, next: &Config) -> ConfigImpact {
    let mut impact = ConfigImpact::default();

    if previous.modules != next.modules {
        impact.layout_changed = true;
    }

    if previous.appearance != next.appearance {
        impact.appearance_changed = true;
    }

    if previous.appearance.workspace_colors != next.appearance.workspace_colors
        || previous.appearance.special_workspace_colors != next.appearance.special_workspace_colors
    {
        impact.affected_modules.insert(ModuleName::Workspaces);
    }

    if previous.outputs != next.outputs {
        impact.outputs_changed = true;
    }

    if previous.position != next.position {
        impact.position_changed = true;
    }

    if previous.log_level != next.log_level {
        impact.log_level_changed = true;
    }

    if previous.menu_keyboard_focus != next.menu_keyboard_focus {
        impact.menu_focus_changed = true;
    }

    mark_if_changed(
        &mut impact,
        ModuleName::AppLauncher,
        &previous.app_launcher_cmd,
        &next.app_launcher_cmd
    );
    mark_if_changed(
        &mut impact,
        ModuleName::Clipboard,
        &previous.clipboard_cmd,
        &next.clipboard_cmd
    );
    mark_if_changed(
        &mut impact,
        ModuleName::Updates,
        &previous.updates,
        &next.updates
    );
    mark_if_changed(
        &mut impact,
        ModuleName::Workspaces,
        &previous.workspaces,
        &next.workspaces
    );
    mark_if_changed(
        &mut impact,
        ModuleName::WindowTitle,
        &previous.window_title,
        &next.window_title
    );
    mark_if_changed(
        &mut impact,
        ModuleName::SystemInfo,
        &previous.system,
        &next.system
    );
    mark_if_changed(&mut impact, ModuleName::Cpu, &previous.system, &next.system);
    mark_if_changed(
        &mut impact,
        ModuleName::Memory,
        &previous.system,
        &next.system
    );
    mark_if_changed(
        &mut impact,
        ModuleName::Battery,
        &previous.battery,
        &next.battery
    );
    mark_if_changed(&mut impact, ModuleName::Clock, &previous.clock, &next.clock);
    mark_if_changed(
        &mut impact,
        ModuleName::Clock,
        &previous.weather,
        &next.weather
    );
    mark_if_changed(
        &mut impact,
        ModuleName::ControlCenter,
        &previous.control_center,
        &next.control_center
    );
    mark_if_changed(
        &mut impact,
        ModuleName::MediaPlayer,
        &previous.media_player,
        &next.media_player
    );
    mark_if_changed(
        &mut impact,
        ModuleName::KeyboardLayout,
        &previous.keyboard_layout,
        &next.keyboard_layout
    );
    mark_if_changed(
        &mut impact,
        ModuleName::Notifications,
        &previous.notifications,
        &next.notifications
    );

    if previous.custom_modules != next.custom_modules {
        impact.custom_modules_changed = true;
        update_custom_module_impact(&mut impact, &previous.custom_modules, &next.custom_modules);
    }

    impact
}

fn mark_if_changed<T>(impact: &mut ConfigImpact, module: ModuleName, previous: &T, next: &T)
where
    T: PartialEq
{
    if previous != next {
        impact.affected_modules.insert(module);
    }
}

fn update_custom_module_impact(
    impact: &mut ConfigImpact,
    previous: &[CustomModuleDef],
    next: &[CustomModuleDef]
) {
    let previous_map: HashMap<&str, &CustomModuleDef> = previous
        .iter()
        .map(|module| (module.name.as_str(), module))
        .collect();
    let next_map: HashMap<&str, &CustomModuleDef> = next
        .iter()
        .map(|module| (module.name.as_str(), module))
        .collect();

    for (name, module) in &next_map {
        let needs_update = previous_map
            .get(name)
            .is_none_or(|current| *current != *module);

        if needs_update {
            impact
                .affected_modules
                .insert(ModuleName::Custom((*name).to_string()));
        }
    }

    for name in previous_map.keys() {
        if !next_map.contains_key(name) {
            impact
                .affected_modules
                .insert(ModuleName::Custom((*name).to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use hydebar_proto::config::{ModuleDef, NotificationSource};

    use super::*;

    /// A desktop theme switch arrives as a reload where only the appearance
    /// moved; it must not read as a reason to restart module listeners.
    #[test]
    fn a_recolour_does_not_move_module_registration() {
        let previous = Config::default();
        let mut next = Config::default();
        next.appearance.opacity = 0.5;

        let impact = compute_impact(&previous, &next);

        assert!(impact.appearance_changed);
        assert!(!impact.moves_module_registration());
    }

    #[test]
    fn a_layout_change_moves_module_registration() {
        let previous = Config::default();
        let mut next = Config::default();
        next.modules.left.push(ModuleDef::Single(ModuleName::Tray));

        assert!(compute_impact(&previous, &next).moves_module_registration());
    }

    #[test]
    fn a_module_whose_configuration_moved_moves_registration() {
        let previous = Config::default();
        let mut next = Config::default();
        next.clock.format = "%H:%M".to_owned();

        let impact = compute_impact(&previous, &next);

        assert!(impact.affects_module(&ModuleName::Clock));
        assert!(impact.moves_module_registration());
    }

    /// Claiming or releasing the notification bus happens at registration
    /// time, so a change of source has to reach it.
    #[test]
    fn a_notification_source_change_moves_registration() {
        let previous = Config::default();
        let mut next = Config::default();
        next.notifications.source = NotificationSource::Builtin;

        let impact = compute_impact(&previous, &next);

        assert!(impact.affects_module(&ModuleName::Notifications));
        assert!(impact.moves_module_registration());
    }
}
