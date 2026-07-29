//! Configuration file watcher.

mod events;
pub(crate) mod interpret;
pub(crate) mod load;
mod recipe;

#[cfg(test)]
mod tests;

pub use events::ConfigEvent;
pub(crate) use events::{Event, WatchLoopOutcome, WatchedEvent};
pub use recipe::subscription;
