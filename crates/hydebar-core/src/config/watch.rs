//! Configuration file watcher.

mod events;
mod interpret;
mod load;
mod recipe;

#[cfg(test)]
mod tests;

pub use events::ConfigEvent;
pub(self) use events::{Event, WatchLoopOutcome, WatchedEvent};
pub use recipe::subscription;
