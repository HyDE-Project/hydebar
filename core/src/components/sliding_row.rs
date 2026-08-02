//! A row whose children glide to their new places instead of jumping.
//!
//! When the bar's arrangement changes, every block is given a key and the
//! row remembers where each key last stood. For as long as the caller's
//! transition is travelling, a surviving block is drawn — and hit — at a
//! position interpolated between its old seat and its new one, so a layout
//! switch reads as furniture sliding across the shelf rather than as a cut.
//!
//! The seams follow the widget's duties: [`builder`] assembles the row,
//! [`state`] is what it remembers between frames, [`layout`] seats the
//! children, [`draw`] paints them where the slide holds them, [`events`]
//! delivers input there too, and [`widget`] ties the pieces to the tree.

mod builder;
mod draw;
mod events;
mod layout;
mod state;
mod widget;

pub use self::builder::SlidingRow;
