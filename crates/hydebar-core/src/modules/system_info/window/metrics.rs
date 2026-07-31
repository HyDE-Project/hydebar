//! Every size of the window, and the room the whole of it takes.
//!
//! The box is sized from these same constants the drawing uses, so a
//! size changed here moves the drawing and the measurement together.

use hydebar_proto::config::SystemModuleConfig;

use super::{
    super::data::SystemInfoData,
    model::{self, Row, Section}
};
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

/// Width of the content column alone.
pub(super) fn column_width() -> f32 {
    scale::scaled(WIDTH)
}

/// Width the menu box needs, box padding included.
pub fn content_width(font_size: f32) -> f32 {
    scale::scaled(WIDTH + 2.0 * OUTER_PADDING)
        + 2.0 * crate::menu::MENU_PADDING_EM * font_size
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
    let gaps = scale::scaled(ROW_GAP) * inner_gaps as f32;

    title + note + rows + gaps
}

fn footnotes_height(footnotes: &[String]) -> f32 {
    if footnotes.is_empty() {
        return 0.0;
    }

    let rule = 1.0;
    let heading = line(SECTION_TITLE_SIZE);
    let lines = footnotes.len() as f32 * line(NOTE_SIZE);
    let gaps = scale::scaled(FOOT_GAP) * (footnotes.len() as f32 + 1.0);

    rule + heading + lines + gaps
}

/// Height the window of one section needs.
pub fn section_window_height(section: Option<&Section>) -> f32 {
    let body = section.map_or(0.0, section_height);

    body + scale::scaled(2.0 * OUTER_PADDING)
}

/// Height the menu content needs for the readouts it currently
/// shows.
pub fn content_height(data: &SystemInfoData, config: &SystemModuleConfig) -> f32 {
    let sections = model::sections(data);
    let footnotes = model::footnotes(data, config);

    let title = line(TITLE_SIZE);
    let rule = 1.0;
    let body: f32 = sections.iter().map(section_height).sum();
    let blocks = 2 + sections.len() + usize::from(!footnotes.is_empty());
    let gaps = scale::scaled(SECTION_GAP) * (blocks as f32 - 1.0);
    let padding = scale::scaled(2.0 * OUTER_PADDING);

    title + rule + body + footnotes_height(&footnotes) + gaps + padding
}
