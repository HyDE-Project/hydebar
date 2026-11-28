//! State of the updates module: what the bar knows, said as data.
//!
//! [`message`] holds what the module answers to and the payloads those
//! answers carry, and [`schedule`] is the one runner that fills them. The
//! rooms around them hold the behaviour: [`queries`] answers what the menu
//! asks, [`transitions`] folds messages in, [`runs`] spawns the two long
//! commands and [`lifecycle`] ties the module to the bar.

mod failures;
mod hyde_clone;
mod lifecycle;
pub(super) mod message;
mod queries;
mod runs;
mod schedule;
mod transitions;

use std::sync::Arc;

use tokio::runtime::Handle;

pub use self::message::{HydeSnapshot, Message, Update};
use self::schedule::Schedule;
use crate::ModuleEventSender;

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum CheckState {
    #[default]
    Checking,
    Ready,
    /// No check can be run here, so the bar has no update count to show.
    Unavailable
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "the open lists and the run outcomes are independent switches, not one state machine"
)]
/// Bar entry counting what is waiting to be installed.
#[derive(Default)]
pub struct Updates {
    state:                    CheckState,
    pending:                  Vec<Update>,
    /// Whether the list of what is waiting is open.
    pub is_updates_list_open: bool,
    is_hyde_list_open:        bool,
    hyde:                     Option<HydeSnapshot>,
    hyde_clone:               Option<Arc<str>>,
    hyde_branch:              Option<Arc<str>>,
    hyde_updating:            bool,
    hyde_failed:              bool,
    hyde_log:                 Vec<String>,
    applying:                 bool,
    apply_failed:             bool,
    apply_log:                Vec<String>,
    update_command:           Option<Arc<str>>,
    sender:                   Option<ModuleEventSender<Message>>,
    runtime:                  Option<Handle>,
    schedule:                 Option<Schedule>,
    shown_count:              crate::components::crossfade::Crossfade
}

impl std::fmt::Debug for Updates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updates")
            .field("state", &self.state)
            .field("pending", &self.pending)
            .field("is_updates_list_open", &self.is_updates_list_open)
            .field("is_hyde_list_open", &self.is_hyde_list_open)
            .field("hyde", &self.hyde)
            .field("hyde_clone", &self.hyde_clone)
            .field("hyde_branch", &self.hyde_branch)
            .field("hyde_updating", &self.hyde_updating)
            .field("hyde_failed", &self.hyde_failed)
            .field("hyde_log", &self.hyde_log)
            .field("applying", &self.applying)
            .field("apply_failed", &self.apply_failed)
            .field("apply_log", &self.apply_log)
            .field("update_command", &self.update_command)
            .field("sender", &self.sender)
            .field("runtime", &self.runtime)
            .field("schedule", &self.schedule)
            .field("shown_count", &self.shown_count)
            .finish()
    }
}

impl Clone for Updates {
    fn clone(&self) -> Self {
        Self {
            state:                self.state.clone(),
            pending:              self.pending.clone(),
            is_updates_list_open: self.is_updates_list_open,
            is_hyde_list_open:    self.is_hyde_list_open,
            hyde:                 self.hyde.clone(),
            hyde_clone:           self.hyde_clone.clone(),
            hyde_branch:          self.hyde_branch.clone(),
            hyde_updating:        self.hyde_updating,
            hyde_failed:          self.hyde_failed,
            hyde_log:             self.hyde_log.clone(),
            applying:             self.applying,
            apply_failed:         self.apply_failed,
            apply_log:            self.apply_log.clone(),
            update_command:       self.update_command.clone(),
            sender:               self.sender.clone(),
            runtime:              self.runtime.clone(),
            schedule:             None,
            shown_count:          self.shown_count.clone()
        }
    }
}

#[cfg(test)]
mod tests;
