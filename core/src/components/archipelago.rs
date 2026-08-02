//! Islands drawn under wherever the icons actually are.
//!
//! The pill behind a group of modules is not a box the modules live in —
//! it is painted every frame around the places the modules currently
//! stand. At rest that reproduces the configured islands exactly. While a
//! rearrangement travels, every module glides from its old seat to its
//! new one carrying a pill of its own, and pills of modules that draw
//! near each other fuse into one island, then part again as they pass —
//! no icon is ever bare, and islands form under the arriving furniture.
//!
//! The seams follow the widget's duties: [`builder`] assembles the strip,
//! [`layout`] seats the modules, [`draw`] paints pills and modules,
//! [`events`] delivers input where things are drawn, and [`widget`] ties
//! the pieces to the tree.

mod builder;
mod draw;
mod events;
mod layout;
mod widget;

pub use self::builder::{Archipelago, PillPaint};
/// The shared book of seats, reused from the flip machinery.
pub use super::flip::FlipMemo;
