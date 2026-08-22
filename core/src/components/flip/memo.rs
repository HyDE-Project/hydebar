//! The shared book of seats every gliding block reads and writes.

use std::collections::HashMap;

use iced_core::Rectangle;

/// The shared book of seats: where every key is, and where it came from.
#[derive(Debug, Default)]
pub struct FlipMemo {
    /// The whole seat every key rests at, as drawn on the latest frame.
    ///
    /// The rectangle rather than the position alone: anything drawing the way
    /// a block came — the trail behind it above all — needs to know how big
    /// the block is as well as where it sits.
    live: HashMap<u64, Rectangle>,
    /// The whole seats the current journey departs from.
    ///
    /// The seat rather than its left edge: a block leaves the strip from the
    /// place the layout gave it there, and anything drawing the way it came
    /// has to start from that place — its middle, its width and all — rather
    /// than from a corner of the very different shape it becomes.
    from: HashMap<u64, Rectangle>
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
    pub fn record(&mut self, key: u64, seat: Rectangle) {
        self.live.insert(key, seat);
    }

    /// The seats every key rested at on the latest frame.
    #[must_use]
    pub const fn seats(&self) -> &HashMap<u64, Rectangle> {
        &self.live
    }

    /// The departure seats of the journey in flight.
    #[must_use]
    pub const fn from_map(&self) -> &HashMap<u64, Rectangle> {
        &self.from
    }
}
