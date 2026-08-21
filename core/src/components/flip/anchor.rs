//! The wrapper that seats one block in the shared book.

use std::cell::RefCell;

use super::memo::FlipMemo;

/// The share of a descending journey spent falling before it closes in.
///
/// A block is on its own level once this much of its journey is done, and
/// what is left is the move along. Stated for the whole crate because what a
/// block does on arriving at its level — open — is timed off it.
pub const DESCENT: f32 = 0.6;

/// Wraps one block so it journeys between its recorded seats.
pub struct FlipAnchor<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    pub(super) key:            u64,
    /// How far the journey has travelled, one meaning at rest.
    pub(super) progress:       f32,
    /// Whether the journey drops to its level before it moves along.
    ///
    /// A block leaving the strip for a lane beside it would otherwise cut the
    /// corner and cross whatever stands between. Falling first and closing in
    /// afterwards keeps it in the lane it left and the lane it is going to,
    /// and in nothing else.
    pub(super) descends_first: bool,
    /// The height the journey departs from, when it starts on another row.
    ///
    /// The book of seats records where a block stood along its row, which is
    /// all a rearrangement within one row needs. A block leaving the strip
    /// for the canvas below it also has a row to cross, and the row it left
    /// is the same for all of them, so it is stated here rather than
    /// remembered per block.
    pub(super) from_y:         Option<f32>,
    pub(super) memo:           &'a RefCell<FlipMemo>,
    pub(super) content:        iced_core::Element<'a, Message, Theme, Renderer>
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
            from_y: None,
            descends_first: false,
            memo,
            content: content.into()
        }
    }

    /// States the height the journey departs from.
    ///
    /// Left unstated a block travels along its row and nowhere else, which is
    /// what a rearrangement of the strip is.
    #[must_use]
    pub const fn departing_from(mut self, y: f32) -> Self {
        self.from_y = Some(y);
        self
    }

    /// Has the journey drop to its level first and close in afterwards.
    #[must_use]
    pub const fn descending_first(mut self) -> Self {
        self.descends_first = true;
        self
    }

    /// The drawing offset for the current frame, given the resting seat.
    pub(super) fn offset(&self, at: iced_core::Point) -> iced_core::Vector {
        if self.progress >= 1.0 {
            return iced_core::Vector::ZERO;
        }

        let (fallen, closed) = if self.descends_first {
            (
                (self.progress / DESCENT).clamp(0.0, 1.0),
                ((self.progress - DESCENT) / (1.0 - DESCENT)).clamp(0.0, 1.0)
            )
        } else {
            (self.progress, self.progress)
        };

        let x = self
            .memo
            .borrow()
            .from_map()
            .get(&self.key)
            .map_or(0.0, |from| (from - at.x) * (1.0 - closed));

        let y = self
            .from_y
            .map_or(0.0, |from| (from - at.y) * (1.0 - fallen));

        iced_core::Vector::new(x, y)
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
