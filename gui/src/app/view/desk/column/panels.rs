//! What a module has to say once it has the room to say all of it.

use hydebar_core::config::ModuleName;

use super::super::{super::super::state::App, readings};

impl App {
    /// What a module has to say once it has the room to say all of it.
    ///
    /// One arm per module, and every arm answers from the state that module
    /// already keeps: the desk samples nothing of its own. A module that has
    /// not answered yet, or has nothing longer to say than its strip entry,
    /// yields no panel and stands on the canvas as it stands on the strip.
    pub(super) fn desk_panels(&self, module: &ModuleName) -> Vec<readings::Panel> {
        let machine = self.system_info.data();

        match module {
            ModuleName::SystemInfo => [
                readings::system(machine),
                readings::cooling(machine),
                readings::processor(machine),
                readings::graphics(machine),
                readings::memory(machine),
                readings::storage(machine),
                readings::network(machine)
            ]
            .into_iter()
            .flatten()
            .collect(),
            ModuleName::Cpu => readings::processor(machine).into_iter().collect(),
            ModuleName::CpuTemp => [
                readings::cpu_temperature(machine),
                readings::cooling(machine)
            ]
            .into_iter()
            .flatten()
            .collect(),
            ModuleName::GpuTemp => readings::graphics(machine).into_iter().collect(),
            ModuleName::Memory => readings::memory(machine).into_iter().collect(),
            ModuleName::Battery => readings::battery(self).into_iter().collect(),
            ModuleName::Updates => readings::updates(self).into_iter().collect(),
            ModuleName::Notifications => readings::notifications(self).into_iter().collect(),
            ModuleName::Privacy => readings::privacy(self).into_iter().collect(),
            ModuleName::KeyboardLayout => readings::keyboard(self).into_iter().collect(),
            ModuleName::Weather => readings::weather(self).into_iter().collect(),
            ModuleName::Tray => readings::tray(self).into_iter().collect(),
            ModuleName::Themes => readings::theme(self).into_iter().collect(),
            ModuleName::ControlCenter => [
                readings::session_idle(self),
                readings::sound(self),
                readings::link(self),
                readings::radio(self),
                readings::screen(self)
            ]
            .into_iter()
            .flatten()
            .collect(),
            ModuleName::IdleInhibitor => readings::session_idle(self).into_iter().collect(),
            ModuleName::Audio => readings::sound(self).into_iter().collect(),
            ModuleName::Network => readings::link(self).into_iter().collect(),
            ModuleName::Bluetooth => readings::radio(self).into_iter().collect(),
            ModuleName::Brightness => readings::screen(self).into_iter().collect(),
            ModuleName::Workspaces => readings::workspaces(self).into_iter().collect(),
            ModuleName::WindowTitle | ModuleName::Taskbar => {
                readings::windows(self).into_iter().collect()
            }
            ModuleName::MediaPlayer => readings::playing(self).into_iter().collect(),
            ModuleName::KeyboardSubmap => readings::submap(self).into_iter().collect(),
            ModuleName::Custom(name) => readings::own(self, name).into_iter().collect(),
            _ => Vec::new()
        }
    }
}
