//! Button widget forked from `iced` that reports the position it was pressed
//! at and dispatches the left, right and middle mouse buttons independently.

mod builder;
mod draw;
mod events;
mod press;
mod state;
mod widget;

use iced::{Padding, core::Layout};

pub use self::{
    builder::{PositionButton, position_button},
    press::ButtonUIRef
};

/// The layout of the single child every button lays out.
///
/// The widget's own `layout` always produces exactly one child node; should
/// the tree ever disagree, drawing within the button's own bounds is a
/// visible glitch where a panic would be a dead bar.
fn content_layout(layout: Layout<'_>) -> Layout<'_> {
    layout.children().next().unwrap_or(layout)
}

/// The default [`Padding`] of a [`PositionButton`].
pub(crate) const DEFAULT_PADDING: Padding = Padding {
    top:    5.0,
    bottom: 5.0,
    right:  10.0,
    left:   10.0
};
