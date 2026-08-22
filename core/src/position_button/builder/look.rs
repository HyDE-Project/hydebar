//! What the button is to look like: its size, its padding, its style.

use iced::{
    Length, Padding,
    id::Id,
    widget::button::{Catalog, Status, Style, StyleFn}
};

use super::PositionButton;

impl<'a, Message, Theme, Renderer> PositionButton<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
    Theme: Catalog
{
    /// Sets the width of the [`Button`].
    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Button`].
    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the [`Padding`] of the [`Button`].
    #[must_use]
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets whether the contents of the [`Button`] should be clipped on
    /// overflow.
    #[must_use]
    pub const fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Sets the style of the [`Button`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the [`Id`] of the [`Button`].
    #[must_use]
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }
}
