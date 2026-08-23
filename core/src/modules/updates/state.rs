//! State of the updates module: what the bar knows, said as data.
//!
//! The rooms around it hold the behaviour: [`queries`] answers what the
//! menu asks, [`transitions`] folds messages in, [`runs`] spawns the two
//! long commands and [`lifecycle`] ties the module to the bar.

use std::sync::Arc;

use tokio::runtime::Handle;

use crate::ModuleEventSender;

mod failures;
mod hyde_clone;
mod lifecycle;
mod queries;
mod runs;
mod schedule;
mod transitions;

use schedule::Schedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub(super) package: String,
    pub(super) from:    String,
    pub(super) to:      String
}

impl Update {
    /// Name of the package the update is for.
    #[must_use]
    pub fn package(&self) -> &str {
        &self.package
    }

    /// Version installed now, and the one it is going to.
    #[must_use]
    pub fn versions(&self) -> (&str, &str) {
        (&self.from, &self.to)
    }
}

/// What one look at the `HyDE` clone reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydeSnapshot {
    /// Version the clone describes itself as.
    pub(crate) version: String,
    /// Subjects of the upstream commits the clone has not taken yet.
    pub(crate) commits: Vec<String>
}

/// What the updates entry answers to.
#[derive(Debug, Clone)]
pub enum Message {
    /// The check finished, and this is what is waiting.
    UpdatesCheckCompleted(Vec<Update>),
    /// A check ran but could not be trusted; what is known already stands.
    CheckFailed,
    /// The configured check cannot be run on this machine.
    UpdatesUnavailable,
    /// The package update ended, well or badly.
    UpdateFinished {
        /// Whether the installation failed.
        failed: bool
    },
    /// The last lines the running package update printed.
    UpdateLog(Vec<String>),
    /// Open or close the list of what is waiting.
    ToggleUpdatesList,
    /// Check for updates now.
    CheckNow,
    /// Apply the configured update command, narrating into the window.
    Update,
    /// The `HyDE` clone was compared against upstream.
    HydeChecked(HydeSnapshot),
    /// Open or close the desktop's own update list.
    ToggleHydeList,
    /// Bring the `HyDE` clone up to date, narrating into the window.
    UpdateHyde,
    /// The last lines the running `HyDE` update printed.
    HydeUpdateLog(Vec<String>),
    /// The `HyDE` update ended, well or badly.
    HydeUpdateFinished {
        /// Whether the update failed.
        failed: bool
    }
}

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
