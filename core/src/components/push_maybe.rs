//! `push_maybe` for the bridge's row and column.
//!
//! The layer-shell bridge re-exports the upstream widgets without the
//! convenience the fork carried, and forty call sites read better with it
//! than with an `if let` around every optional child. Restated here once,
//! exactly as upstream spelled it.

use iced::widget::{Column, Row};

/// Pushes a child that may not be there, leaving the container unchanged
/// when it is not.
pub trait PushMaybe<'a, Message, Theme, Renderer>: Sized {
    #[must_use]
    fn push_maybe(
        self,
        child: Option<impl Into<iced_core::Element<'a, Message, Theme, Renderer>>>
    ) -> Self;
}

impl<'a, Message, Theme, Renderer> PushMaybe<'a, Message, Theme, Renderer>
    for Row<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn push_maybe(
        self,
        child: Option<impl Into<iced_core::Element<'a, Message, Theme, Renderer>>>
    ) -> Self {
        match child {
            Some(child) => self.push(child),
            None => self
        }
    }
}

impl<'a, Message, Theme, Renderer> PushMaybe<'a, Message, Theme, Renderer>
    for Column<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn push_maybe(
        self,
        child: Option<impl Into<iced_core::Element<'a, Message, Theme, Renderer>>>
    ) -> Self {
        match child {
            Some(child) => self.push(child),
            None => self
        }
    }
}
