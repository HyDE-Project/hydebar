//! Button widget forked from `iced` that reports the position it was pressed
//! at and dispatches the left, right and middle mouse buttons independently.

mod builder;
mod draw;
mod events;
mod press;
mod state;
mod widget;

use iced::Padding;

pub use self::{
    builder::{PositionButton, position_button},
    press::ButtonUIRef
};

/// The default [`Padding`] of a [`PositionButton`].
pub(crate) const DEFAULT_PADDING: Padding = Padding {
    top:    5.0,
    bottom: 5.0,
    right:  10.0,
    left:   10.0
};
