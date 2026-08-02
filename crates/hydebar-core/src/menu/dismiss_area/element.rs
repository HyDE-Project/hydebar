//! The dismiss area element and its construction.
//!
//! The wrapper owns the wrapped bar content and the two messages it reports —
//! one for the press landing, one for the press completing; how it watches
//! those presses lives in [`super::widget`].

/// Press state of a [`DismissArea`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct State {
    /// Whether the press in flight started on this area.
    pub(super) pressed: bool
}

/// Wrapper reporting the presses that land on the element it wraps.
///
/// Unlike [`iced::widget::mouse_area`] it also reports a press a child widget
/// consumed: a module button swallowing its own press must not hide from the
/// bar that a press happened, or pressing that module would leave an open menu
/// behind.
///
/// The press and its completion are reported apart on purpose. A press only
/// arms the dismissal, and the module the press lands on is given the whole
/// click to take it over, so switching from one menu to the next never flashes
/// the menu surface off and on again.
pub struct DismissArea<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    pub(super) content:    iced_core::Element<'a, Message, Theme, Renderer>,
    pub(super) on_press:   Message,
    pub(super) on_release: Message
}

impl<Message, Theme, Renderer> std::fmt::Debug for DismissArea<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DismissArea").finish_non_exhaustive()
    }
}

/// Wraps `content` so every press on it is reported.
///
/// `on_press` is published when a press lands on the wrapped element and
/// `on_release` once that same press completes on it.
pub fn dismiss_area<'a, Message, Theme, Renderer>(
    content: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>,
    on_press: Message,
    on_release: Message
) -> DismissArea<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    DismissArea {
        content: content.into(),
        on_press,
        on_release
    }
}

impl<'a, Message, Theme, Renderer> From<DismissArea<'a, Message, Theme, Renderer>>
    for iced_core::Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: iced_core::Renderer + 'a
{
    fn from(area: DismissArea<'a, Message, Theme, Renderer>) -> Self {
        Self::new(area)
    }
}
