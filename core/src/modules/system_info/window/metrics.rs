//! Every size of the window, and the room the whole of it takes.
//!
//! The box is sized from these same constants the drawing uses, so a
//! size changed here moves the drawing and the measurement together.

use super::model::{self, Row, Section};
use crate::components::scale;

/// Window title size, in pixels of the reference theme.
pub(super) const TITLE_SIZE: f32 = 18.0;

/// Section title size, in pixels of the reference theme.
pub(super) const SECTION_TITLE_SIZE: f32 = 12.0;

/// Reading label and value size, in pixels of the reference theme.
pub(super) const ROW_SIZE: f32 = 13.0;

/// Source note and footnote size, in pixels of the reference theme.
pub(super) const NOTE_SIZE: f32 = 11.0;

/// Height of a usage meter track, in pixels of the reference theme.
pub(super) const METER_HEIGHT: f32 = 5.0;

/// Gap between a reading and the meter under it.
pub(super) const METER_GAP: f32 = 4.0;

/// Gap between two rows of one section.
pub(super) const ROW_GAP: f32 = 6.0;

/// Gap between the title, the sections and the footnotes.
pub(super) const SECTION_GAP: f32 = 10.0;

/// Gap between the footnote lines.
pub(super) const FOOT_GAP: f32 = 4.0;

/// Padding of the whole column, in pixels of the reference theme.
pub(super) const OUTER_PADDING: f32 = 4.0;

/// Width of the content column, in pixels of the reference theme.
pub(super) const WIDTH: f32 = 300.0;

/// Height one line of text at `size` occupies at the stock line
/// height.
fn line(size: f32) -> f32 {
    scale::scaled(size) * 1.3
}

/// A count of rows or lines as the f32 the layout math runs in.
#[expect(
    clippy::cast_precision_loss,
    reason = "row and line counts stay far below f32's exact integer range"
)]
const fn count(value: usize) -> f32 {
    value as f32
}

/// Width of the content column alone.
pub(super) fn column_width() -> f32 {
    scale::scaled(WIDTH)
}

/// Width the menu box needs, box padding included.
pub fn content_width(font_size: f32) -> f32 {
    (2.0 * crate::menu::MENU_PADDING_EM).mul_add(
        font_size,
        scale::scaled(2.0f32.mul_add(OUTER_PADDING, WIDTH))
    )
}

fn row_height(row: &Row) -> f32 {
    match row {
        Row::Fact {
            ..
        } => line(ROW_SIZE),
        Row::Meter {
            ..
        } => line(ROW_SIZE) + scale::scaled(METER_GAP + METER_HEIGHT)
    }
}

fn section_height(section: &Section) -> f32 {
    let title = line(SECTION_TITLE_SIZE);
    let note = section.note.as_ref().map_or(0.0, |_| line(NOTE_SIZE));
    let rows: f32 = section.rows.iter().map(row_height).sum();
    let inner_gaps = section.rows.len() + usize::from(section.note.is_some());
    let gaps = scale::scaled(ROW_GAP) * count(inner_gaps);

    title + note + rows + gaps
}

fn footnotes_height(footnotes: &[String]) -> f32 {
    if footnotes.is_empty() {
        return 0.0;
    }

    let rule = 1.0;
    let heading = line(SECTION_TITLE_SIZE);
    let lines = count(footnotes.len()) * line(NOTE_SIZE);
    let gaps = scale::scaled(FOOT_GAP) * (count(footnotes.len()) + 1.0);

    rule + heading + lines + gaps
}

/// Height the window of one section needs.
pub fn section_window_height(section: Option<&Section>) -> f32 {
    let body = section.map_or(0.0, section_height);

    body + scale::scaled(2.0 * OUTER_PADDING)
}

/// Height the menu content needs for an already built model.
pub fn content_height_of(sections: &[model::Section], footnotes: &[String]) -> f32 {
    let title = line(TITLE_SIZE);
    let rule = 1.0;
    let body: f32 = sections.iter().map(section_height).sum();
    let blocks = 2 + sections.len() + usize::from(!footnotes.is_empty());
    let gaps = scale::scaled(SECTION_GAP) * (count(blocks) - 1.0);
    let padding = scale::scaled(2.0 * OUTER_PADDING);

    title + rule + body + footnotes_height(footnotes) + gaps + padding
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::components::icons::Icons;

    const TOLERANCE: f32 = 0.01;

    fn fact() -> Row {
        Row::Fact {
            label: "Kernel".to_owned(),
            value: "7.1.5".to_owned()
        }
    }

    fn meter() -> Row {
        Row::Meter {
            label:   "Load".to_owned(),
            value:   "34%".to_owned(),
            percent: 34
        }
    }

    fn section(rows: Vec<Row>, note: Option<String>) -> Section {
        Section {
            icon: Icons::Cpu,
            title: "Processor",
            note,
            rows
        }
    }

    #[test]
    fn a_meter_row_stands_taller_than_a_fact_row() {
        let extra = row_height(&meter()) - row_height(&fact());

        assert!((extra - scale::scaled(METER_GAP + METER_HEIGHT)).abs() < TOLERANCE);
    }

    #[test]
    fn each_row_adds_its_height_and_one_gap() {
        let one = section_height(&section(vec![fact()], None));
        let two = section_height(&section(vec![fact(), fact()], None));

        let expected = row_height(&fact()) + scale::scaled(ROW_GAP);

        assert!((two - one - expected).abs() < TOLERANCE);
    }

    #[test]
    fn a_note_costs_a_line_and_a_gap() {
        let bare = section_height(&section(vec![fact()], None));
        let noted = section_height(&section(vec![fact()], Some("k10temp".to_owned())));

        let expected = line(NOTE_SIZE) + scale::scaled(ROW_GAP);

        assert!((noted - bare - expected).abs() < TOLERANCE);
    }

    #[test]
    fn an_absent_section_costs_only_the_padding() {
        let empty = section_window_height(None);
        let filled = section_window_height(Some(&section(vec![fact()], None)));

        assert!((empty - scale::scaled(2.0 * OUTER_PADDING)).abs() < TOLERANCE);
        assert!(filled > empty);
    }

    #[test]
    fn no_footnotes_take_no_room() {
        assert!(footnotes_height(&[]).abs() < f32::EPSILON);
    }

    #[test]
    fn each_footnote_adds_a_line_and_a_gap() {
        let one = footnotes_height(&["GPU — no device".to_owned()]);
        let two = footnotes_height(&[
            "GPU — no device".to_owned(),
            "Swap usage — no swap".to_owned()
        ]);

        let expected = line(NOTE_SIZE) + scale::scaled(FOOT_GAP);

        assert!(one > 0.0);
        assert!((two - one - expected).abs() < TOLERANCE);
    }

    #[test]
    fn the_content_grows_with_sections_and_footnotes() {
        let one_section = [section(vec![fact()], None)];
        let two_sections = [section(vec![fact()], None), section(vec![meter()], None)];
        let footnote = ["Swap usage — no swap".to_owned()];

        assert!(
            content_height_of(&two_sections, &[]) > content_height_of(&one_section, &[])
        );
        assert!(
            content_height_of(&one_section, &footnote) > content_height_of(&one_section, &[])
        );
    }

    #[test]
    fn the_menu_box_is_wider_than_its_content_column() {
        assert!(content_width(10.0) > column_width());
        assert!(content_width(20.0) > content_width(10.0));
    }
}
