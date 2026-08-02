//! Management of the last known valid configuration and reload impact.
//!
//! A reload flows through here in two halves: the difference between the old
//! and new state is restated as an impact in [`impact`], and the state itself
//! is held and swapped by the manager in [`state`]. What can go wrong on the
//! way — and what the bar falls back to when it does — lives in [`error`].

mod error;
mod impact;
mod state;

pub use error::{ConfigDegradation, ConfigManagerError, ConfigUpdateError};
pub use impact::ConfigImpact;
pub use state::{ConfigApplied, ConfigManager};
