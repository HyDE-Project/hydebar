//! How much of a column a block takes, and how it is written into it.

use iced::{Element, Length, widget::container};

use super::{super::super::super::state::Message, Ink};

/// The room a block of `rows` lines, each `line` tall, takes when it is open.
///
/// A heading, the rule under it and a line per reading, with the column's own
/// gap between each of them. Stated rather than measured because the room has
/// to be taken before there is anything in it: see [`revealed`].
#[expect(
    clippy::cast_precision_loss,
    reason = "a block holds a handful of rows, far below any precision limit"
)]
pub(super) fn room(rows: usize, line: f32, ink: Ink) -> f32 {
    let heading = ink.size * 1.05 * 1.4;
    let gaps = (rows + 1) as f32 * (ink.size * 0.28);

    (rows as f32).mul_add(line, heading + 1.0 + gaps)
}

/// Opens `shown` from the top inside the room it will need when it is open.
///
/// A block that grew as it opened pushed everything below it down the column,
/// one layout per frame, all the way through the opening — which is the
/// juddering the whole canvas had, and it landed on the lower blocks worst
/// because every block above them was growing at once. The room is taken in
/// full from the first frame instead, and the opening changes only how much
/// of it has been written into. Nothing on the canvas moves while a block
/// opens.
pub(super) fn revealed(
    shown: Element<'_, Message>,
    full: f32,
    bloom: f32
) -> Element<'_, Message> {
    container(
        container(shown)
            .max_height(full * bloom.clamp(0.0, 1.0))
            .clip(true)
    )
    .height(Length::Fixed(full))
    .clip(true)
    .into()
}

/// How tall a picture stands in a block, at the given ink.
///
/// Six lines: a wallpaper says nothing at the height of one, and a block that
/// is mostly picture stops being a reading.
pub(super) fn picture(ink: Ink) -> f32 {
    ink.size * 6.0
}

/// How many lines of the body ink the month grid stands.
///
/// A heading, a row of weekday names and six weeks, each a line and a little,
/// with the grid's own padding around them — measured off the grid itself at
/// the body size and left a little over, because a figure short of the truth
/// does not merely open early, it clips the last week off for good.
pub(super) const MONTH_ROWS: f32 = 22.0;
