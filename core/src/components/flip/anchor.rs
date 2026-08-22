//! The wrapper that seats one block in the shared book.

use std::cell::RefCell;

use super::memo::FlipMemo;

/// The share of a descending journey spent falling.
///
/// A block is on its own level once this much of its journey is done. Stated
/// for the whole crate because what a block does on arriving at its level —
/// open — is timed off it.
pub const DESCENT: f32 = 0.8;

/// When the move along the lane begins, as a share of the journey.
///
/// Not at the very start: every block of a column leaves the same row, so for
/// the first instant they are all at one height and only their own widths
/// keep them apart. The fall puts a level between them first — by this much
/// of the journey the nearest pair is further apart than either is tall — and
/// only then does anything move sideways.
const CLOSING_FROM: f32 = 0.22;

/// When the move along the lane is over, as a share of the journey.
///
/// Well before the fall is, and that is the whole point of the figure. A
/// block that closed in after it had landed spent the last and most read
/// stretch of its journey travelling sideways, and the bar came in from the
/// edges of the screen instead of coming down off the strip. Ending the
/// sideways move early leaves the descent as the last thing the eye follows,
/// which is the one thing this journey is meant to say.
const CLOSING_TO: f32 = 0.5;

/// Where a block stands on the way from `from` to `at`, at `progress`.
///
/// The one place the path is worked out, so anything that draws the way a
/// block came — the trail behind it — traces the very line the block flies
/// rather than a second guess at it.
#[must_use]
pub fn offset_of(
    progress: f32,
    descends_first: bool,
    from_x: Option<f32>,
    from_y: Option<f32>,
    at: iced_core::Point
) -> iced_core::Vector {
    if progress >= 1.0 {
        return iced_core::Vector::ZERO;
    }

    let (fallen, closed) = if descends_first {
        (
            (progress / DESCENT).clamp(0.0, 1.0),
            ((progress - CLOSING_FROM) / (CLOSING_TO - CLOSING_FROM)).clamp(0.0, 1.0)
        )
    } else {
        (progress, progress)
    };

    let x = from_x.map_or(0.0, |from| (from - at.x) * (1.0 - closed));
    let y = from_y.map_or(0.0, |from| (from - at.y) * (1.0 - fallen));

    iced_core::Vector::new(x, y)
}

/// Wraps one block so it journeys between its recorded seats.
pub struct FlipAnchor<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    pub(super) key:            u64,
    /// How far the journey has travelled, one meaning at rest.
    pub(super) progress:       f32,
    /// Whether the journey closes in along its lane before it has landed.
    ///
    /// A block leaving the strip for a lane beside it would otherwise cut the
    /// corner and cross whatever stands between. It falls out of the row it
    /// shared with its neighbours first, closes in along its lane while the
    /// fall is still on, and finishes the journey coming straight down into
    /// its place.
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

    /// Has the journey close in along its lane early and land straight down.
    #[must_use]
    pub const fn descending_first(mut self) -> Self {
        self.descends_first = true;
        self
    }

    /// The drawing offset for the current frame, given the resting seat.
    pub(super) fn offset(&self, at: iced_core::Point) -> iced_core::Vector {
        let from_x = self
            .memo
            .borrow()
            .from_map()
            .get(&self.key)
            .map(|seat| seat.x);

        offset_of(self.progress, self.descends_first, from_x, self.from_y, at)
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
