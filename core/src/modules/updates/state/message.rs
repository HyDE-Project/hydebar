//! What the updates module answers to, and the payloads those answers
//! carry.
//!
//! These are the plain facts the rooms around them move about: the check
//! command produces [`Update`]s, the `HyDE` comparison produces a
//! [`HydeSnapshot`], and both arrive as a [`Message`].

/// One package waiting to be installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub(in crate::modules::updates) package: String,
    pub(in crate::modules::updates) from:    String,
    pub(in crate::modules::updates) to:      String
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
