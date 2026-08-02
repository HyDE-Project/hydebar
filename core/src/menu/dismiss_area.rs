//! Bar side of the rule that a press outside a menu dismisses it.
//!
//! The backdrop of a menu only covers the screen the bar leaves free, so a
//! press landing on the bar itself never reaches it. This wrapper reports
//! those presses instead, which is what lets the one rule hold over the whole
//! screen instead of stopping at the edge of the bar.
//!
//! The element itself and its construction live in [`element`], its widget
//! behaviour in [`widget`].

mod element;
mod widget;

pub use element::{DismissArea, dismiss_area};
