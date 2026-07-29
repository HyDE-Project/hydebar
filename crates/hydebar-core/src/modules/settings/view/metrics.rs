//! Width a piece of the settings window is expected to take.
//!
//! The layout engine reports sizes only after it has laid a widget out, which
//! is too late to decide how wide the window should be. The window therefore
//! estimates the width of its own rows from the text it is about to draw, and
//! the longest row becomes the width of the window.

/// Width one character of a label takes, in multiples of the text size.
///
/// The window is drawn in the monospaced theme font, so a single advance
/// describes every glyph.
const GLYPH_ADVANCE_EM: f32 = 0.66;

/// Slack added to a measured row, in multiples of the text size.
///
/// The estimate is deliberately a little generous: a row that asks for a few
/// pixels too many is invisible, while a row that asks for too few pushes its
/// label into the controls beside it.
pub(super) const ROW_SLACK_EM: f32 = 2.5;

/// Width a button spends on padding beside its label, in multiples of the text
/// size.
pub(super) const BUTTON_PADDING_EM: f32 = 2.4;

/// Width `label` takes when drawn at `font_size`.
#[must_use]
pub(super) fn text_width(label: &str, font_size: f32) -> f32 {
    label.chars().count() as f32 * GLYPH_ADVANCE_EM * font_size
}

/// Width a button carrying `label` takes at `font_size`.
#[must_use]
pub(super) fn button_width(label: &str, font_size: f32) -> f32 {
    text_width(label, font_size) + BUTTON_PADDING_EM * font_size
}

/// Width a row of buttons takes, gaps included.
#[must_use]
pub(super) fn button_row_width<'a, I>(labels: I, font_size: f32, gap: f32) -> f32
where
    I: IntoIterator<Item = &'a str>
{
    let mut width = 0.0_f32;
    let mut count = 0.0_f32;

    for label in labels {
        width += button_width(label, font_size);
        count += 1.0;
    }

    width + gap * (count - 1.0).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_longer_label_takes_more_room() {
        assert!(text_width("ab", 10.0) < text_width("abcd", 10.0));
    }

    #[test]
    fn a_larger_text_size_takes_more_room() {
        assert!(text_width("abc", 10.0) < text_width("abc", 20.0));
    }

    #[test]
    fn a_button_adds_its_padding_to_the_label() {
        assert_eq!(
            button_width("abc", 10.0),
            text_width("abc", 10.0) + BUTTON_PADDING_EM * 10.0
        );
    }

    #[test]
    fn a_row_counts_the_gaps_between_its_buttons() {
        let single = button_row_width(["one"], 10.0, 4.0);
        let pair = button_row_width(["one", "two"], 10.0, 4.0);

        assert_eq!(pair, single + button_width("two", 10.0) + 4.0);
    }

    #[test]
    fn an_empty_row_takes_no_room() {
        assert_eq!(button_row_width(std::iter::empty(), 10.0, 4.0), 0.0);
    }
}
