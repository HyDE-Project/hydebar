//! The button itself, and the two rooms its builder is grouped into.
//!
//! The button is one type with one job — report where it was pressed. Its
//! builder is grouped by what a caller is setting: [`look`] is what the button
//! is to look like, [`presses`] is what it is to answer to. What is left here
//! is the button, its making and its handing over to the widget tree.

mod look;
mod presses;

use iced::{Length, Padding, id::Id, widget::button::Catalog};

use super::{DEFAULT_PADDING, press::OnPress};

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
