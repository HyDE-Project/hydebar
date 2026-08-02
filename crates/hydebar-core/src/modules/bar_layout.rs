//! Bar entry stepping and picking the `HyDE` bar layout in force.
//!
//! The desktop owns the layout roster and the record of the one in force;
//! the module asks, and the desktop does the rest — the bar itself follows
//! through the state watch it already keeps. The mouse speaks the same
//! dialect as the wallpaper entry: the side buttons step, the middle
//! button opens the picker.
//!
//! One folder, four rooms: [`roster`] reads the desktop's layouts and the
//! record of the one in force, [`state`] folds messages in and dispatches
//! commands, [`view`] draws the picker and [`module`] wires the module to
//! the bar. The root holds the state the rooms share.

mod module;
mod roster;
mod state;
mod view;

/// One layout the desktop offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutEntry {
    /// Name the desktop lists the layout under.
    pub name:   String,
    /// Whether this is the layout in force.
    pub active: bool
}

/// Messages the module answers.
#[derive(Debug, Clone)]
pub enum Message {
    /// Ask the desktop for the next layout.
    Next,
    /// Ask the desktop for the previous layout.
    Previous,
    /// Report that the layout change has ended.
    Changed {
        /// Why the desktop refused, if it did.
        failure: Option<String>
    },
    /// Deliver the desktop's layout roster to the picker.
    Listed(Vec<LayoutEntry>),
    /// Ask the desktop to arrange the bar by the named layout.
    Pick(String),
    /// Advance the loading indicator by one frame.
    Tick
}

/// State of the bar layout module.
#[derive(Debug, Clone, Default)]
pub struct BarLayout {
    /// Layouts the desktop offers, while the picker shows them.
    entries: Vec<LayoutEntry>,
    /// Whether the roster is being read right now.
    loading: bool,
    /// Frame the loading indicator is on.
    spinner: crate::modules::themes::Spinner
}

impl BarLayout {
    /// Builds the module.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the picker has nothing to show yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether the roster is being read right now.
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        self.loading
    }
}
