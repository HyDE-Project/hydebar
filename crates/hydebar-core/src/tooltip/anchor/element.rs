//! The tooltip anchor element and its construction.
//!
//! The wrapper owns the wrapped module and the handler that turns a hover
//! into a message; how it reacts to events lives in [`super::widget`].

use crate::position_button::ButtonUIRef;

/// Hover state of a [`TooltipAnchor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct State {
    /// Whether the pointer was last seen over the wrapped element.
    pub(super) is_hovered:     bool,
    /// Whether the pointer is on this surface at all.
    ///
    /// A surface keeps reporting the last position the pointer had on it after
    /// the pointer has left, so leaving has to be remembered: without it the
    /// next redraw would read the stale position and call the module hovered
    /// again.
    pub(super) pointer_inside: bool
}

/// Wrapper reporting when the pointer enters and leaves the module it wraps.
///
/// The bar cannot draw the tooltip itself, so it publishes the hover instead
/// and lets the tooltip surface render it.
pub struct TooltipAnchor<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    pub(super) content:  iced_core::Element<'a, Message, Theme, Renderer>,
    pub(super) on_hover: Box<dyn Fn(Option<ButtonUIRef>) -> Message + 'a>
}

impl<Message, Theme, Renderer> std::fmt::Debug for TooltipAnchor<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TooltipAnchor").finish_non_exhaustive()
    }
}

/// Wraps `content` so hovering it publishes the message `on_hover` builds.
///
/// The handler receives the on-screen placement of the wrapped element while
/// the pointer rests on it, and [`None`] once the pointer leaves.
pub fn tooltip_anchor<'a, Message, Theme, Renderer>(
    content: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>,
    on_hover: impl Fn(Option<ButtonUIRef>) -> Message + 'a
) -> TooltipAnchor<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    TooltipAnchor {
        content:  content.into(),
        on_hover: Box::new(on_hover)
    }
}

impl<'a, Message, Theme, Renderer> From<TooltipAnchor<'a, Message, Theme, Renderer>>
    for iced_core::Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced_core::Renderer + 'a
{
    fn from(anchor: TooltipAnchor<'a, Message, Theme, Renderer>) -> Self {
        Self::new(anchor)
    }
}
