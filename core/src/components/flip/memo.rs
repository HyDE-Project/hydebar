//! The shared book of seats every gliding block reads and writes.

use std::collections::HashMap;

/// The shared book of seats: where every key is, and where it came from.
#[derive(Debug, Default)]
pub struct FlipMemo {
    /// Absolute positions as they were drawn on the latest frame.
    live: HashMap<u64, f32>,
    /// Absolute positions the current journey departs from.
    from: HashMap<u64, f32>
}

impl FlipMemo {
    /// Freezes the live seats as the departure points of a new journey.
    ///
    /// Called at the moment a rearrangement is adopted, before the next
    /// frame lays the new arrangement out. The live book is taken, not
    /// copied: the coming frames restate every surviving seat, and a seat
    /// nobody restates belonged to a surface or module that is gone —
    /// taking is what keeps the book from hoarding the dead.
    pub fn depart(&mut self) {
        self.from = std::mem::take(&mut self.live);
    }

    /// Writes the seat `key` rests at on the current frame.
    pub fn record(&mut self, key: u64, x: f32) {
        self.live.insert(key, x);
    }

    /// The departure seats of the journey in flight.
    #[must_use]
    pub const fn from_map(&self) -> &HashMap<u64, f32> {
        &self.from
    }
}
