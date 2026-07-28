//! The [`Centerbox`] container and the builder methods configuring it.

use iced::{Alignment, Element, Length, Padding, Pixels};

/// A container that distributes its contents horizontally.
#[allow(missing_debug_implementations)]
pub struct Centerbox<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    pub(super) spacing:     f32,
    pub(super) padding:     Padding,
    pub(super) width:       Length,
    pub(super) height:      Length,
    pub(super) align_items: Alignment,
    pub(super) children:    [Element<'a, Message, Theme, Renderer>; 3]
}

impl<'a, Message, Theme, Renderer> Centerbox<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer
{
    /// Creates an empty [`Centerbox`].
    pub fn new(children: [Element<'a, Message, Theme, Renderer>; 3]) -> Self {
        Centerbox {
            spacing: 0.0,
            padding: Padding::ZERO,
            width: Length::Shrink,
            height: Length::Shrink,
            align_items: Alignment::Start,
            children
        }
    }

    /// Sets the horizontal spacing _between_ elements.
    ///
    /// Custom margins per element do not exist in iced. You should use this
    /// method instead! While less flexible, it helps you keep spacing between
    /// elements consistent.
    pub fn spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.spacing = amount.into().0;
        self
    }

    /// Sets the [`Padding`] of the [`Centerbox`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the width of the [`Centerbox`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Centerbox`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the vertical alignment of the contents of the [`Centerbox`] .
    pub fn align_items(mut self, align: Alignment) -> Self {
        self.align_items = align;
        self
    }
}

impl<'a, Message, Theme, Renderer> From<Centerbox<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a
{
    fn from(row: Centerbox<'a, Message, Theme, Renderer>) -> Self {
        Self::new(row)
    }
}
