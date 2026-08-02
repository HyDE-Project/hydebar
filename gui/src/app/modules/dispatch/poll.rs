//! Sampling of the pollable modules on their declared cadences.

use hydebar_core::{attention::PollSchedule, config::ModuleName};
use log::error;

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
}
