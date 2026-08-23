//! Sampling of the pollable modules on their declared cadences.

use hydebar_core::{attention::PollSchedule, config::ModuleName};
use log::error;

use crate::app::state::App;

impl App {
    /// The cadences `module_name` declared, if it can be polled at all.
    ///
    /// Thin bar entries answer with the schedule of the module that owns
    /// their data: the standalone network entry with the control centre's,
    /// the processor and memory entries with the system monitor's. Whichever
    /// of them the user is looking at, the shared readouts are what want
    /// refreshing.
    pub(crate) fn module_poll_schedule(&self, module_name: &ModuleName) -> Option<PollSchedule> {
        self.module_owner(module_name)?.poll_schedule()
    }

    /// Takes one sample of `module_name`.
    pub(crate) fn poll_module(&mut self, module_name: &ModuleName) {
        let ctx = self.module_context.clone();

        let Some(owner) = self.module_owner_mut(module_name) else {
            return;
        };

        if let Err(err) = owner.poll(&ctx) {
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
