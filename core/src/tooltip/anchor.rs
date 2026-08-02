//! Wrapper widget that publishes the hover the tooltip surface renders.
//!
//! The element itself and its construction live in [`element`], its widget
//! behaviour in [`widget`].

mod element;
mod widget;

pub use element::{TooltipAnchor, tooltip_anchor};
