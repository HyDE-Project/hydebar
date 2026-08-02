//! The per-module markers: which module a moved section belongs to.

use hydebar_proto::config::{Config, ModuleName};

use super::ConfigImpact;

/// Marks every module whose own configuration section moved.
///
/// One flat comparison per module keeps every impact visible in one place;
/// the shared sections are marked for every consumer they feed (the system
/// section for the standalone readouts, the weather section for the clock
/// hosting the readout).
pub(super) fn mark_module_configs(impact: &mut ConfigImpact, previous: &Config, next: &Config) {
    mark_if_changed(
        impact,
        ModuleName::AppLauncher,
        &previous.app_launcher_cmd,
        &next.app_launcher_cmd
    );
    mark_if_changed(
        impact,
        ModuleName::Clipboard,
        &previous.clipboard_cmd,
        &next.clipboard_cmd
    );
    mark_if_changed(impact, ModuleName::Updates, &previous.updates, &next.updates);
    mark_if_changed(
        impact,
        ModuleName::Workspaces,
        &previous.workspaces,
        &next.workspaces
    );
    mark_if_changed(
        impact,
        ModuleName::WindowTitle,
        &previous.window_title,
        &next.window_title
    );
    mark_if_changed(
        impact,
        ModuleName::SystemInfo,
        &previous.system,
        &next.system
    );
    mark_if_changed(impact, ModuleName::Cpu, &previous.system, &next.system);
    mark_if_changed(impact, ModuleName::Memory, &previous.system, &next.system);
    mark_if_changed(
        impact,
        ModuleName::Battery,
        &previous.battery,
        &next.battery
    );
    mark_if_changed(impact, ModuleName::Clock, &previous.clock, &next.clock);
    mark_if_changed(impact, ModuleName::Clock, &previous.weather, &next.weather);
    mark_if_changed(
        impact,
        ModuleName::ControlCenter,
        &previous.control_center,
        &next.control_center
    );
    mark_if_changed(
        impact,
        ModuleName::MediaPlayer,
        &previous.media_player,
        &next.media_player
    );
    mark_if_changed(
        impact,
        ModuleName::KeyboardLayout,
        &previous.keyboard_layout,
        &next.keyboard_layout
    );
    mark_if_changed(
        impact,
        ModuleName::Notifications,
        &previous.notifications,
        &next.notifications
    );
}

fn mark_if_changed<T>(impact: &mut ConfigImpact, module: ModuleName, previous: &T, next: &T)
where
    T: PartialEq
{
    if previous != next {
        impact.affected_modules.insert(module);
    }
}
