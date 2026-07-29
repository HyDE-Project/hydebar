//! The one set of sizes the settings window is drawn and measured from.
//!
//! Every page used to pick its own gaps and its own text scales, so the same
//! kind of row looked different depending on which tab it sat on. The sizes now
//! live here and nowhere else: a page states which shape it wants, never how
//! large that shape is. The measuring code reads the very same constants, so
//! the width and height the window asks the compositor for describe the window
//! it actually draws.

/// Text size of a control, in multiples of the page text size.
///
/// Controls read a shade smaller than the label beside them so a row scans as a
/// label first and its buttons second, and so a long row of choices still fits
/// a window sized for the labels.
pub(crate) const CONTROL_SCALE: f32 = 0.85;

/// Text size of a caption or an aside, in multiples of the page text size.
///
/// A caption names a group rather than carrying a value, so it is drawn smaller
/// than anything it introduces and never competes with it for attention.
pub(crate) const CAPTION_SCALE: f32 = 0.75;

/// Height of a line of text, in multiples of its own text size.
///
/// The layout engine reports a laid out height only after the fact, which is
/// too late to size the window, so a row is measured from this instead.
pub(crate) const LINE_HEIGHT_EM: f32 = 1.3;

/// Padding a button keeps around its label: above and below, then left and
/// right, in multiples of the button's own text size.
///
/// Stated in ems rather than pixels so a bar configured with a larger font gets
/// proportionally larger buttons instead of cramped ones.
pub(crate) const BUTTON_PADDING_EM: [f32; 2] = [0.5, 1.0];

/// Padding a chip keeps around its label, in multiples of its own text size.
///
/// Tighter than a button because a row of chips is a picture of the bar rather
/// than a row of controls, and it has to stay readable when a section holds
/// many of them.
pub(crate) const CHIP_PADDING_EM: [f32; 2] = [0.35, 0.7];

/// Gap between the controls of a single row, in multiples of the page text
/// size.
pub(crate) const ROW_GAP_EM: f32 = 0.8;

/// Gap between the rows of a page, in multiples of the page text size.
///
/// Wider than the gap inside a row, so a row reads as one thing and the page
/// reads as a list of them.
pub(crate) const PAGE_GAP_EM: f32 = 1.0;

/// Gap between the sections of a page, in multiples of the page text size.
///
/// Wider again than the gap between two rows, which is the whole of the page
/// rhythm: controls sit closest, rows further apart, sections further still. A
/// page that spaced its sections like its rows would read as one long list with
/// headings sprinkled through it.
pub(crate) const SECTION_GAP_EM: f32 = 1.4;

/// Padding a page keeps inside the window, in multiples of the page text size.
///
/// Small on purpose: the menu box already pads the whole window, and this only
/// keeps a card's outline from sitting flush against that padding.
pub(crate) const PAGE_PADDING_EM: f32 = 0.25;

/// Text size of a section heading, in multiples of the page text size.
///
/// A heading is the largest thing on a page and the only text drawn at the full
/// page size on a line of its own, which is what makes a section readable as a
/// section without a rule or a colour of its own.
pub(crate) const SECTION_TITLE_SCALE: f32 = 1.0;

/// Rows a section spends on its heading.
///
/// The heading sits on its own line above the section, so a section is one row
/// taller than what it holds. Pages count with this rather than with a literal,
/// so a heading that grew to two lines would be changed in one place.
pub(crate) const SECTION_TITLE_ROWS: f32 = 1.0;

/// Gap between the header, the tab row and the page, in multiples of the page
/// text size.
pub(crate) const WINDOW_GAP_EM: f32 = 1.0;

/// Gaps the window column is measured with.
///
/// Two of them sit between the header, the tabs and the page; the third stands
/// in for the padding the menu box draws around the whole column, which the
/// page itself cannot see.
pub(crate) const WINDOW_GAP_COUNT: f32 = 3.0;

/// Gap between items that belong to the same group, in multiples of the page
/// text size.
pub(crate) const GROUP_GAP_EM: f32 = 0.4;

/// Gap between a caption and what it captions, in multiples of the page text
/// size.
pub(crate) const CAPTION_GAP_EM: f32 = 0.3;

/// Padding inside a card or an outlined box, in multiples of the page text
/// size.
pub(crate) const CARD_PADDING_EM: f32 = 0.6;

/// Share of the window opacity a card's fill is drawn at.
///
/// A card is a shade of the surface it sits on rather than a surface of its
/// own, so it is drawn part transparent: filling it at the full window opacity
/// would make it read as a second window laid over the first.
pub(crate) const CARD_FILL_ALPHA: f32 = 0.5;

/// Corner radius of a card, a chip or an outlined box, in multiples of the page
/// text size.
///
/// One radius for every surface, so the window never mixes two roundnesses.
pub(crate) const CORNER_RADIUS_EM: f32 = 0.5;

/// Thickness of the outline drawn around a box, in pixels.
///
/// A hairline is a hairline at any text size, so this one is not an em.
pub(crate) const BORDER_WIDTH: f32 = 1.0;

/// Width of the label column of a settings row, in multiples of the page text
/// size.
///
/// Fixed rather than sized to each label, so the controls of every row on every
/// tab start at the same x and the pages read as one grid. Wide enough for the
/// longest label any page draws, which the page tests hold it to.
pub(crate) const LABEL_WIDTH_EM: f32 = 13.0;

/// Width of an icon glyph, in multiples of the page text size.
///
/// An icon is drawn from a font rather than from an image, so it takes one full
/// text size across whatever the icon happens to be; the header is measured
/// with this instead of leaving the icon out and coming up short.
pub(crate) const ICON_WIDTH_EM: f32 = 1.0;

/// Height one row of a page takes, in multiples of the page text size.
///
/// Derived from the tallest thing a row can hold — a button — rather than
/// guessed, so the measured height of a page tracks any change to the button
/// padding instead of drifting away from it.
pub(crate) const ROW_HEIGHT_EM: f32 =
    CONTROL_SCALE * (LINE_HEIGHT_EM + 2.0 * BUTTON_PADDING_EM[0]);

/// Text size a control is drawn at on a page whose text is `font_size`.
#[must_use]
pub(crate) fn control_size(font_size: f32) -> f32 {
    CONTROL_SCALE * font_size
}

/// Text size a caption is drawn at on a page whose text is `font_size`.
#[must_use]
pub(crate) fn caption_size(font_size: f32) -> f32 {
    CAPTION_SCALE * font_size
}

/// Gap between the controls of a row.
#[must_use]
pub(crate) fn row_gap(font_size: f32) -> f32 {
    ROW_GAP_EM * font_size
}

/// Gap between the rows of a page.
#[must_use]
pub(crate) fn page_gap(font_size: f32) -> f32 {
    PAGE_GAP_EM * font_size
}

/// Gap between the sections of a page.
#[must_use]
pub(crate) fn section_gap(font_size: f32) -> f32 {
    SECTION_GAP_EM * font_size
}

/// Padding a page keeps inside the window.
#[must_use]
pub(crate) fn page_padding(font_size: f32) -> f32 {
    PAGE_PADDING_EM * font_size
}

/// Text size a section heading is drawn at.
#[must_use]
pub(crate) fn section_title_size(font_size: f32) -> f32 {
    SECTION_TITLE_SCALE * font_size
}

/// Gap between the header, the tabs and the page.
#[must_use]
pub(crate) fn window_gap(font_size: f32) -> f32 {
    WINDOW_GAP_EM * font_size
}

/// Gap between items of the same group.
#[must_use]
pub(crate) fn group_gap(font_size: f32) -> f32 {
    GROUP_GAP_EM * font_size
}

/// Gap between a caption and what it captions.
#[must_use]
pub(crate) fn caption_gap(font_size: f32) -> f32 {
    CAPTION_GAP_EM * font_size
}

/// Width of the label column of a settings row.
#[must_use]
pub(crate) fn label_width(font_size: f32) -> f32 {
    LABEL_WIDTH_EM * font_size
}

/// Padding a card or an outlined box keeps around its contents.
#[must_use]
pub(crate) fn card_padding(font_size: f32) -> f32 {
    CARD_PADDING_EM * font_size
}

/// Room a card or an outlined box takes on top of its contents.
///
/// Both edges of the padding, so a page measuring a boxed row adds what the box
/// actually costs rather than half of it.
#[must_use]
pub(crate) fn card_overhead(font_size: f32) -> f32 {
    2.0 * card_padding(font_size)
}

/// Corner radius of a card, a chip or an outlined box.
#[must_use]
pub(crate) fn corner_radius(font_size: f32) -> f32 {
    CORNER_RADIUS_EM * font_size
}

/// Width an icon glyph takes beside text of `font_size`.
#[must_use]
pub(crate) fn icon_width(font_size: f32) -> f32 {
    ICON_WIDTH_EM * font_size
}

/// Height one row of a page takes.
#[must_use]
pub(crate) fn row_height(font_size: f32) -> f32 {
    ROW_HEIGHT_EM * font_size
}

/// Distance from the top of one row to the top of the next.
#[must_use]
pub(crate) fn row_pitch(font_size: f32) -> f32 {
    row_height(font_size) + page_gap(font_size)
}

/// Height a page of `rows` rows takes, its own padding included.
///
/// Counted as a whole pitch per row rather than as rows plus the gaps between
/// them: the trailing gap is what keeps the last row off the bottom edge of the
/// window, and it also covers the extra room a section gap takes over a row gap
/// without every page having to count its sections twice.
#[must_use]
pub(crate) fn page_height(rows: f32, font_size: f32) -> f32 {
    rows * row_pitch(font_size) + 2.0 * page_padding(font_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_as_tall_as_the_button_it_holds() {
        let font_size = 16.0;
        let button = control_size(font_size) * LINE_HEIGHT_EM
            + 2.0 * BUTTON_PADDING_EM[0] * control_size(font_size);

        assert!((row_height(font_size) - button).abs() < f32::EPSILON);
    }

    #[test]
    fn a_control_is_smaller_than_the_label_beside_it() {
        assert!(control_size(16.0) < 16.0);
    }

    #[test]
    fn a_caption_is_smaller_than_a_control() {
        assert!(caption_size(16.0) < control_size(16.0));
    }

    #[test]
    fn rows_are_spaced_wider_than_the_controls_inside_one() {
        assert!(page_gap(16.0) > group_gap(16.0));
    }

    #[test]
    fn sections_are_spaced_wider_than_the_rows_inside_one() {
        assert!(section_gap(16.0) > page_gap(16.0));
    }

    #[test]
    fn a_section_heading_is_the_largest_text_on_a_page() {
        let font_size = 16.0;

        assert!(section_title_size(font_size) >= control_size(font_size));
        assert!(section_title_size(font_size) > caption_size(font_size));
    }

    #[test]
    fn a_page_of_more_rows_is_taller() {
        assert!(page_height(4.0, 16.0) > page_height(3.0, 16.0));
    }

    #[test]
    fn a_row_pitch_is_the_row_plus_the_gap_below_it() {
        let font_size = 16.0;

        assert!(
            (row_pitch(font_size) - (row_height(font_size) + page_gap(font_size))).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn every_size_grows_with_the_text_it_is_measured_in() {
        assert!(label_width(20.0) > label_width(16.0));
        assert!(row_height(20.0) > row_height(16.0));
        assert!(caption_size(20.0) > caption_size(16.0));
    }
}
