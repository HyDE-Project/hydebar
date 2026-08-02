use iced::{
    Length, Padding,
    core::mouse,
    id::Id,
    widget::button::{Catalog, Status, Style, StyleFn}
};

use super::{
    DEFAULT_PADDING,
    press::{ButtonUIRef, OnPress}
};

/// Button reporting the on-screen position it was pressed at.
pub struct PositionButton<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer,
    Theme: Catalog
{
    pub(super) content:         iced_core::Element<'a, Message, Theme, Renderer>,
    pub(super) on_press:        Option<OnPress<'a, Message>>,
    pub(super) on_right_press:  Option<OnPress<'a, Message>>,
    pub(super) on_middle_press: Option<OnPress<'a, Message>>,
    pub(super) id:              Id,
    pub(super) width:           Length,
    pub(super) height:          Length,
    pub(super) padding:         Padding,
    pub(super) clip:            bool,
    pub(super) class:           Theme::Class<'a>
}

impl<Message, Theme, Renderer> std::fmt::Debug for PositionButton<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
    Theme: Catalog
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PositionButton")
            .field("id", &self.id)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("padding", &self.padding)
            .field("clip", &self.clip)
            .finish_non_exhaustive()
    }
}

impl<'a, Message, Theme, Renderer> PositionButton<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
    Theme: Catalog
{
    pub fn new(content: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>) -> Self {
        let content = content.into();
        let size = content.as_widget().size_hint();

        PositionButton {
            content,
            id: Id::unique(),
            on_press: None,
            on_right_press: None,
            on_middle_press: None,
            width: size.width.fluid(),
            height: size.height.fluid(),
            padding: DEFAULT_PADDING,
            clip: false,
            class: Theme::default()
        }
    }

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

    /// Sets the message that will be produced when the [`Button`] is pressed.
    ///
    /// Unless `on_press` is called, the [`Button`] will be disabled.
    #[must_use]
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(OnPress::Message(on_press));
        self
    }

    #[must_use]
    pub fn on_press_with_position(
        mut self,
        on_press: impl Fn(ButtonUIRef) -> Message + 'a
    ) -> Self {
        self.on_press = Some(OnPress::MessageWithPosition(Box::new(on_press)));
        self
    }

    /// Sets the message produced when the [`Button`] is pressed with the right
    /// mouse button.
    ///
    /// Right presses are dispatched independently of the left ones, so a button
    /// may carry either, both or neither handler.
    #[must_use]
    pub fn on_right_press(mut self, on_press: Message) -> Self {
        self.on_right_press = Some(OnPress::Message(on_press));
        self
    }

    /// Sets the message produced when the [`Button`] is pressed with the right
    /// mouse button, built from the on-screen position of the button.
    #[must_use]
    pub fn on_right_press_with_position(
        mut self,
        on_press: impl Fn(ButtonUIRef) -> Message + 'a
    ) -> Self {
        self.on_right_press = Some(OnPress::MessageWithPosition(Box::new(on_press)));
        self
    }

    /// Sets the message produced when the [`Button`] is pressed with the middle
    /// mouse button.
    ///
    /// Middle presses are dispatched independently of the left ones, so a
    /// button may carry either, both or neither handler.
    #[must_use]
    pub fn on_middle_press(mut self, on_press: Message) -> Self {
        self.on_middle_press = Some(OnPress::Message(on_press));
        self
    }

    /// Sets the message produced when the [`Button`] is pressed with the middle
    /// mouse button, built from the on-screen position of the button.
    #[must_use]
    pub fn on_middle_press_with_position(
        mut self,
        on_press: impl Fn(ButtonUIRef) -> Message + 'a
    ) -> Self {
        self.on_middle_press = Some(OnPress::MessageWithPosition(Box::new(on_press)));
        self
    }

    /// Reports whether any mouse button carries a handler.
    pub(super) const fn is_pressable(&self) -> bool {
        self.on_press.is_some() || self.on_right_press.is_some() || self.on_middle_press.is_some()
    }

    /// Borrows the handler the given mouse button carries, if any.
    pub(super) const fn handler(&self, button: mouse::Button) -> Option<&OnPress<'a, Message>> {
        match button {
            mouse::Button::Right => self.on_right_press.as_ref(),
            mouse::Button::Middle => self.on_middle_press.as_ref(),
            _ => self.on_press.as_ref()
        }
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

impl<'a, Message, Theme, Renderer> From<PositionButton<'a, Message, Theme, Renderer>>
    for iced_core::Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: iced_core::Renderer + 'a
{
    fn from(button: PositionButton<'a, Message, Theme, Renderer>) -> Self {
        Self::new(button)
    }
}

/// Builds a [`PositionButton`] wrapping the given content.
pub fn position_button<'a, Message, Theme, Renderer>(
    content: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>
) -> PositionButton<'a, Message, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: iced_core::Renderer
{
    PositionButton::new(content)
}
