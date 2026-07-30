//! Width a piece of the settings window is expected to take.
//!
//! The layout engine reports sizes only after it has laid a widget out, which
//! is too late to decide how wide the window should be. The window therefore
//! estimates the width of its own rows from the text it is about to draw, and
//! the longest row becomes the width of the window.
//!
//! Every estimate here is built out of [`super::style`], which is the same set
//! of sizes the widgets are drawn from: a measurement that carried its own copy
//! of a padding would quietly stop matching the window the moment that padding
//! changed.

use super::style;

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
pub(crate) const ROW_SLACK_EM: f32 = 2.5;

/// Width `label` takes when drawn at `font_size`.
#[must_use]
pub(crate) fn text_width(label: &str, font_size: f32) -> f32 {
    label.chars().count() as f32 * GLYPH_ADVANCE_EM * font_size
}

/// Width a button carrying `label` takes at `font_size`.
///
/// `font_size` is the size the button's own label is drawn at, not the page
/// text size, because the padding is stated in the button's own ems.
#[must_use]
pub(crate) fn button_width(label: &str, font_size: f32) -> f32 {
    text_width(label, font_size) + 2.0 * style::BUTTON_PADDING_EM[1] * font_size
}

/// Width a chip carrying `label` takes at `font_size`.
///
/// A chip is padded more tightly than a button, so measuring one as the other
/// would fit fewer of them per row than the window actually draws.
#[must_use]
pub(crate) fn chip_width(label: &str, font_size: f32) -> f32 {
    text_width(label, font_size) + 2.0 * style::CHIP_PADDING_EM[1] * font_size
}

/// Width the live indicator adds in front of a reported value.
///
/// One glyph of the icon font, drawn at the control text size, plus the gap
/// that separates it from the value beside it. Reserved by the pages that can
/// show one so a window measured while nothing is running is still wide enough
/// the moment something starts.
#[must_use]
pub(crate) fn indicator_width(font_size: f32) -> f32 {
    style::icon_width(style::control_size(font_size)) + style::row_gap(font_size)
}

/// Width a row of buttons takes, gaps included.
#[must_use]
pub(crate) fn button_row_width<'a, I>(labels: I, font_size: f32, gap: f32) -> f32
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

/// Width a settings row takes: the label column, one gap, then `controls`.
///
/// Every page measures its rows through this one function, so a page cannot ask
/// for a width its own row shape would not produce.
#[must_use]
pub(crate) fn row_width<'a, I>(controls: I, font_size: f32) -> f32
where
    I: IntoIterator<Item = &'a str>
{
    style::label_width(font_size)
        + style::row_gap(font_size)
        + button_row_width(
            controls,
            style::control_size(font_size),
            style::row_gap(font_size)
        )
}

/// Width a row reporting `value` beside its label takes.
///
/// A reported value is plain text rather than a button, so it carries no
/// padding of its own.
#[must_use]
pub(crate) fn status_row_width(value: &str, font_size: f32) -> f32 {
    style::label_width(font_size)
        + style::row_gap(font_size)
        + text_width(value, style::control_size(font_size))
}

/// Width of the one cell every chip of a grid is drawn in.
///
/// The widest label decides for everyone. Cells of one width are what holds a
/// grid's shape still while a theme switch changes the font or the active
/// name: chips packed by their own widths re-measure and re-wrap on every
/// such change, and a grid that rearranges under an open menu reads as
/// breakage, not as a theme.
#[must_use]
pub(crate) fn chip_cell_width(labels: &[String], font_size: f32) -> f32 {
    let chip_font = style::control_size(font_size);

    labels
        .iter()
        .map(|label| chip_width(label, chip_font))
        .fold(0.0_f32, f32::max)
}

/// Groups `labels` into the rows of chips they fill when laid out `available`
/// wide at the page text size `font_size`.
///
/// Every chip occupies the same cell — see [`chip_cell_width`] — so the
/// grouping depends on how many cells fit a row, never on which labels happen
/// to share it.
#[must_use]
pub(crate) fn wrap_chips_into_rows(
    labels: &[String],
    available: f32,
    font_size: f32,
    gap: f32
) -> Vec<Vec<usize>> {
    let cell = chip_cell_width(labels, font_size);
    let per_row = (((available + gap) / (cell + gap)).floor() as usize).max(1);

    (0..labels.len())
        .collect::<Vec<usize>>()
        .chunks(per_row)
        .map(<[usize]>::to_vec)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_that_fits_stays_a_single_row() {
        let labels = vec!["Clock".to_owned(), "Tray".to_owned()];

        assert_eq!(
            wrap_chips_into_rows(&labels, 1000.0, 10.0, 4.0),
            vec![vec![0, 1]]
        );
    }

    #[test]
    fn a_long_grid_wraps_onto_several_rows() {
        let labels = (0..8).map(|i| format!("Module{i}")).collect::<Vec<_>>();

        let rows = wrap_chips_into_rows(&labels, 200.0, 10.0, 4.0);

        assert!(rows.len() > 1);
        assert_eq!(rows.iter().map(Vec::len).sum::<usize>(), labels.len());
    }

    #[test]
    fn an_entry_wider_than_the_row_still_gets_a_row() {
        let labels = vec!["AnExtremelyLongModuleName".to_owned()];

        assert_eq!(
            wrap_chips_into_rows(&labels, 10.0, 10.0, 4.0),
            vec![vec![0]]
        );
    }

    /// The cell is the widest chip, so the shape of the grid cannot depend on
    /// which labels happen to share a row — every row holds the same number
    /// of cells, and renaming or restyling a label never re-wraps the rest.
    #[test]
    fn rows_hold_the_same_number_of_cells_whatever_the_labels() {
        let short = (0..6).map(|i| format!("A{i}")).collect::<Vec<_>>();
        let mut mixed = short.clone();
        mixed[0] = "A very long theme name".to_owned();

        let per_row = |rows: Vec<Vec<usize>>| rows.first().map(Vec::len);
        let cell = chip_cell_width(&mixed, 16.0);
        let width = cell * 3.0 + 2.0 * 6.0;

        assert_eq!(
            per_row(wrap_chips_into_rows(&mixed, width, 16.0, 6.0)),
            Some(3)
        );
    }

    #[test]
    fn every_chip_lands_in_exactly_one_row() {
        let labels = (0..7).map(|i| format!("Module{i}")).collect::<Vec<_>>();

        let rows = wrap_chips_into_rows(&labels, 200.0, 16.0, 6.0);

        assert_eq!(rows.iter().map(Vec::len).sum::<usize>(), labels.len());
    }

    #[test]
    fn a_chip_is_narrower_than_a_button_carrying_the_same_label() {
        assert!(chip_width("Clock", 16.0) < button_width("Clock", 16.0));
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
            text_width("abc", 10.0) + 2.0 * style::BUTTON_PADDING_EM[1] * 10.0
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

    #[test]
    fn a_measured_row_reserves_the_shared_label_column() {
        let font_size = 16.0;

        assert!(row_width(["On", "Off"], font_size) > style::label_width(font_size));
        assert!(status_row_width("Nord", font_size) > style::label_width(font_size));
    }

    #[test]
    fn a_row_of_more_controls_is_wider() {
        let font_size = 16.0;

        assert!(row_width(["On", "Off", "Auto"], font_size) > row_width(["On", "Off"], font_size));
    }
}
