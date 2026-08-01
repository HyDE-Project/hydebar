//! The wrapper that seats one block in the shared book.

use std::cell::RefCell;

use super::memo::FlipMemo;

/// Wraps one block so it journeys between its recorded seats.
pub struct FlipAnchor<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    pub(super) key:      u64,
    /// How far the journey has travelled, one meaning at rest.
    pub(super) progress: f32,
    pub(super) memo:     &'a RefCell<FlipMemo>,
    pub(super) content:  iced_core::Element<'a, Message, Theme, Renderer>
}

impl<'a, Message, Theme, Renderer> FlipAnchor<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    /// Wraps `content` under its stable `key`.
    pub fn new(
        key: u64,
        progress: f32,
        memo: &'a RefCell<FlipMemo>,
        content: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>
    ) -> Self {
        Self {
            key,
            progress,
            memo,
            content: content.into()
        }
    }

    /// The drawing offset for the current frame, given the resting seat.
    pub(super) fn offset(&self, x: f32) -> f32 {
        if self.progress >= 1.0 {
            return 0.0;
        }

        self.memo
            .borrow()
            .from_map()
            .get(&self.key)
            .map_or(0.0, |from| (from - x) * (1.0 - self.progress))
    }
}

impl<Message, Theme, Renderer> std::fmt::Debug for FlipAnchor<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlipAnchor")
            .field("key", &self.key)
            .field("progress", &self.progress)
            .finish_non_exhaustive()
    }
}
