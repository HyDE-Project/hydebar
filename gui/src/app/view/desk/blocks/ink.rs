//! The shared drawing vocabulary of the blocks: which edge a column sits on,
//! how far along its journey a block is, and what ink it is drawn in.

use iced::{Alignment, Color};

/// Which edge of the canvas a column is pinned to.
///
/// The columns face outwards, the way the wallpaper's own margins run: the
/// left one reads label first, the right one value first, so both leave the
/// middle of the screen to the hour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app::view::desk) enum Side {
    /// Pinned to the left edge, label before value.
    Leading,
    /// Pinned to the right edge, value before label.
    Trailing,
    /// Standing in the middle, where the centre of the strip was.
    Middle
}

impl Side {
    /// The padding that holds a block `inwards` from its own edge.
    ///
    /// The lane a block stands in: the edge sections push their near blocks
    /// away from the edge they belong to, and the middle one stands where it
    /// is.
    pub(in crate::app::view::desk) fn lane(self, inwards: f32) -> iced::Padding {
        match self {
            Self::Leading => iced::Padding::default().left(inwards),
            Self::Trailing => iced::Padding::default().right(inwards),
            Self::Middle => iced::Padding::default()
        }
    }

    /// How a column of this side lines its content up.
    pub(in crate::app::view::desk) const fn alignment_x(self) -> Alignment {
        match self {
            Self::Leading => Alignment::Start,
            Self::Trailing => Alignment::End,
            Self::Middle => Alignment::Center
        }
    }
}

/// How far a block is along its own journey, in the three figures the
/// drawing of it asks for.
///
/// Carried together because they are one answer: a block is this far across,
/// this far written out and lit this much, all of it read off the same clock
/// at the same instant.
#[derive(Debug, Clone, Copy)]
pub(in crate::app::view::desk) struct Along {
    /// How far it has crossed, one being home.
    pub(in crate::app::view::desk) travel: f32,
    /// How far it has written itself out, zero being not yet begun.
    pub(in crate::app::view::desk) bloom:  f32,
    /// How much of its journey's light it is carrying.
    pub(in crate::app::view::desk) glow:   f32
}

/// Ink of a panel: what its headings, labels and values are drawn in.
#[derive(Debug, Clone, Copy)]
pub(in crate::app::view::desk) struct Ink {
    /// Colour the values are drawn in, the strongest on the canvas.
    pub(in crate::app::view::desk) value: Color,
    /// Size the body lines are drawn at.
    pub(in crate::app::view::desk) size:  f32
}

impl Ink {
    /// Colour of the headings, a step back from the values.
    pub(in crate::app::view::desk) fn heading(self) -> Color {
        self.value.scale_alpha(0.85)
    }

    /// Colour of the labels, the quietest ink of the canvas.
    pub(super) fn label(self) -> Color {
        self.value.scale_alpha(0.55)
    }
}
