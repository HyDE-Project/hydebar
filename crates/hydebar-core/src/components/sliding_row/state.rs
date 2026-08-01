//! What the row remembers between frames: where every key last stood.

/// Where each key stood when the row last rested.
#[derive(Debug, Clone, Default)]
pub(super) struct State {
    /// Positions the settled row assigned, by child key.
    pub(super) settled: std::collections::HashMap<u64, f32>,
    /// Positions the current slide departs from, by child key.
    pub(super) from:    std::collections::HashMap<u64, f32>
}
