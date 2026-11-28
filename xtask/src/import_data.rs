//! Plain data shapes a Waybar configuration is converted into.
//!
//! The orchestrator produces these values and the leaves consume them, so
//! every module that touches them imports them from here rather than from
//! each other.

/// One bar section after migration.
pub enum SectionEntry {
    /// One module, standing on its own island.
    Single(String),
    /// A Waybar pill group, kept as one island.
    Group(Vec<String>)
}

/// A Waybar custom module carried over with its commands.
pub struct CustomEntry {
    /// Module key without the `custom/` prefix.
    pub name:     String,
    /// Command producing the readout.
    pub exec:     Option<String>,
    /// Seconds between runs, if the definition schedules any.
    pub interval: Option<u64>,
    /// Signal offset refreshing on demand, if the definition names one.
    pub signal:   Option<u8>,
    /// Click command, if the definition answers presses.
    pub command:  Option<String>
}

/// Everything a Waybar configuration becomes.
pub struct Converted {
    /// Bar edge, when the source names one this bar knows.
    pub position:        Option<String>,
    /// Compositor layer, when the source names one this bar knows.
    pub layer:           Option<String>,
    /// Sections in bar order, skipping the empty ones.
    pub sections:        Vec<(&'static str, Vec<SectionEntry>)>,
    /// Clock format string, when the source states one.
    pub clock_format:    Option<String>,
    /// Carried-over custom modules, in definition order.
    pub customs:         Vec<CustomEntry>,
    /// Entries with no counterpart, in first-seen order.
    pub unmapped:        Vec<String>,
    /// Scripted customs a native module displaced.
    pub dropped_scripts: Vec<String>
}
