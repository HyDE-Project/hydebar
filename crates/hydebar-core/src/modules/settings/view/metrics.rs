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

/// Height one row of controls takes, in multiples of the text size.
///
/// A button is its label plus padding above and below, and rows are laid out
/// with that height whether they carry a button or only a label.
pub(super) const ROW_HEIGHT_EM: f32 = 2.4;

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

/// Groups `labels` into the rows they fill when laid out `available` wide.
///
/// Wrapping is computed rather than left to the layout engine because the rows
/// have to be counted before they are drawn: the height of the window depends
/// on how many of them there turn out to be. A label wider than the whole row
/// still gets a row of its own, so nothing is silently dropped.
#[must_use]
pub(super) fn wrap_into_rows(
    labels: &[String],
    available: f32,
    font_size: f32,
    gap: f32
) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut used = 0.0;

    for (index, label) in labels.iter().enumerate() {
        let width = button_width(label, font_size);
        let fits = used + width <= available;

        match rows.last_mut() {
            Some(row) if fits => {
                row.push(index);
                used += width + gap;
            }
            _ => {
                rows.push(vec![index]);
                used = width + gap;
            }
        }
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_catalogue_that_fits_stays_a_single_row() {
        let labels = vec!["Clock".to_owned(), "Tray".to_owned()];

        assert_eq!(wrap_into_rows(&labels, 1000.0, 10.0, 4.0), vec![vec![0, 1]]);
    }

    #[test]
    fn a_long_catalogue_wraps_onto_several_rows() {
        let labels = (0..8).map(|i| format!("Module{i}")).collect::<Vec<_>>();

        let rows = wrap_into_rows(&labels, 200.0, 10.0, 4.0);

        assert!(rows.len() > 1);
        assert_eq!(rows.iter().map(Vec::len).sum::<usize>(), labels.len());
    }

    #[test]
    fn an_entry_wider_than_the_row_still_gets_a_row() {
        let labels = vec!["AnExtremelyLongModuleName".to_owned()];

        assert_eq!(wrap_into_rows(&labels, 10.0, 10.0, 4.0), vec![vec![0]]);
    }

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
