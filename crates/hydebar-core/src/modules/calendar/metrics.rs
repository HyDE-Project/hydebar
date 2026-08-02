//! Every size of the view, and the room the whole of it takes.
//!
//! The window is sized from these same constants, so the box always hugs the
//! grid: a size changed here moves the drawing and the measurement together.

use crate::components::scale;

/// Week starts on Monday, the way the reference desktop counts.
pub(super) const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Side of one day cell, in pixels of the reference theme.
pub(super) const CELL: f32 = 36.0;

/// Gap between day cells, in pixels of the reference theme.
pub(super) const CELL_GAP: f32 = 4.0;

/// Month title size, in pixels of the reference theme.
pub(super) const TITLE_SIZE: f32 = 18.0;

/// Weekday header size, in pixels of the reference theme.
pub(super) const WEEKDAY_SIZE: f32 = 12.0;

/// Day number size, in pixels of the reference theme.
pub(super) const DAY_SIZE: f32 = 14.0;

/// Gap between the header, the rule, the weekdays and the grid.
pub(super) const SECTION_GAP: f32 = 8.0;

/// Padding of the whole column, in pixels of the reference theme.
pub(super) const OUTER_PADDING: f32 = 4.0;

/// Vertical room the renderer's stock button padding adds to the header.
const NAV_BUTTON_PADDING: f32 = 10.0;

/// Height one line of text at `size` occupies at the stock line height.
fn scaled_line(size: f32) -> f32 {
    scale::scaled(size) * 1.3
}

/// Width of the day grid alone.
pub(super) fn grid_width() -> f32 {
    scale::scaled(6.0f32.mul_add(CELL_GAP, 7.0 * CELL))
}

/// Width the menu box needs, box padding included.
pub(super) fn content_width(font_size: f32) -> f32 {
    (2.0 * crate::menu::MENU_PADDING_EM).mul_add(
        font_size,
        scale::scaled(2.0f32.mul_add(OUTER_PADDING, 6.0f32.mul_add(CELL_GAP, 7.0 * CELL)))
    )
}

/// Height the menu content needs.
pub(super) fn content_height() -> f32 {
    let header = scaled_line(TITLE_SIZE) + NAV_BUTTON_PADDING;
    let weekdays = scaled_line(WEEKDAY_SIZE);
    let grid = scale::scaled(5.0f32.mul_add(CELL_GAP, 6.0 * CELL));
    let rule = 1.0;
    let spacings = 3.0 * scale::scaled(SECTION_GAP);
    let padding = scale::scaled(2.0 * OUTER_PADDING);

    header + rule + weekdays + grid + spacings + padding
}

#[cfg(test)]
mod tests {
    use super::WEEKDAYS;

    #[test]
    fn the_week_starts_on_monday() {
        assert_eq!(WEEKDAYS.len(), 7);
        assert_eq!(WEEKDAYS[0], "Mon");
        assert_eq!(WEEKDAYS[6], "Sun");
    }
}
