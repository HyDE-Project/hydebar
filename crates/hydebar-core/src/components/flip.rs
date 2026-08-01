//! Every block glides to its new seat across the whole panel.
//!
//! Each module is wrapped in an anchor that records its absolute position
//! on every frame into one shared registry. The moment the arrangement
//! changes, the registry's live positions are frozen into a departure map;
//! for as long as the caller's transition travels, every surviving module
//! is drawn — and hit — between its old seat and its new one, wherever on
//! the panel both happen to be. A module changing islands or sections
//! therefore flies there as one piece instead of reappearing.
//!
//! Three parts: [`memo`] keeps the shared book of seats, [`anchor`] wraps
//! one block under its key, and [`widget`] is where the wrapper meets the
//! widget tree.

mod anchor;
mod memo;
mod widget;

pub use self::{anchor::FlipAnchor, memo::FlipMemo};
