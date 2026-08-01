//! Drawing of the window: sections, aligned value columns and themed
//! usage meters. The meter itself is drawn next door, in [`meter`].

use iced::{
    Alignment, Element, Length, Theme,
    widget::{Column, column, container, row, rule}
};

use super::{
    super::Message,
    metrics::{
        self, FOOT_GAP, NOTE_SIZE, OUTER_PADDING, ROW_GAP, ROW_SIZE, SECTION_GAP,
        SECTION_TITLE_SIZE, TITLE_SIZE
    },
    model::{self, Row, Section}
};
use crate::components::{
    icons::{IconTheme, icon},
    scale,
    text::text
};

mod meter;

use meter::meter_view;

/// Render the module menu from an already built model.
#[must_use]
pub fn build_menu_view<'a>(
    sections: Vec<model::Section>,
    footnotes: Vec<String>,
    icons: &IconTheme
) -> Element<'a, Message> {
    let mut content = Column::new()
        .push(text("System monitor").size(scale::scaled(TITLE_SIZE)))
        .push(rule::horizontal(1));

    for section in sections {
        content = content.push(section_view(section, icons));
    }

    if !footnotes.is_empty() {
        content = content.push(footnotes_view(footnotes));
    }

    content
        .spacing(scale::scaled(SECTION_GAP))
        .padding(scale::scaled(OUTER_PADDING))
        .width(Length::Fixed(metrics::column_width()))
        .into()
}

/// Render the window of one section, for a standalone entry.
///
/// The section is the whole window: its own header names the subject,
/// so the monitor's title and the other sections stay out.
pub fn build_section_window<'a>(
    section: Option<Section>,
    icons: &IconTheme
) -> Element<'a, Message> {
    let mut content = Column::new();

    if let Some(section) = section {
        content = content.push(section_view(section, icons));
    }

    content
        .spacing(scale::scaled(SECTION_GAP))
        .padding(scale::scaled(OUTER_PADDING))
        .width(Length::Fixed(metrics::column_width()))
        .into()
}

fn section_view<'a>(section: Section, icons: &IconTheme) -> Element<'a, Message> {
    let header = container(
        row![
            icon(icons, section.icon).size(scale::scaled(SECTION_TITLE_SIZE)),
            text(section.title).size(scale::scaled(SECTION_TITLE_SIZE))
        ]
        .align_y(Alignment::Center)
        .spacing(scale::icon_gap())
    )
    .style(secondary_text);

    let mut body = Column::new().push(header);

    if let Some(note) = section.note {
        body =
            body.push(container(text(note).size(scale::scaled(NOTE_SIZE))).style(secondary_text));
    }

    for entry in section.rows {
        body = body.push(row_view(entry));
    }

    body.spacing(scale::scaled(ROW_GAP)).into()
}

fn row_view<'a>(entry: Row) -> Element<'a, Message> {
    match entry {
        Row::Fact {
            label,
            value
        } => fact_view(label, value),
        Row::Meter {
            label,
            value,
            percent
        } => column![fact_view(label, value), meter_view(percent)]
            .spacing(scale::scaled(metrics::METER_GAP))
            .into()
    }
}

/// A label on the left, its value on the right edge of the column.
fn fact_view<'a>(label: String, value: String) -> Element<'a, Message> {
    row![
        container(text(label).size(scale::scaled(ROW_SIZE)))
            .style(secondary_text)
            .width(Length::Fill),
        text(value).size(scale::scaled(ROW_SIZE))
    ]
    .align_y(Alignment::Center)
    .into()
}

fn footnotes_view<'a>(footnotes: Vec<String>) -> Element<'a, Message> {
    let mut body = Column::new().push(rule::horizontal(1)).push(
        container(text("Not reported on this machine").size(scale::scaled(SECTION_TITLE_SIZE)))
            .style(secondary_text)
    );

    for entry in footnotes {
        body =
            body.push(container(text(entry).size(scale::scaled(NOTE_SIZE))).style(secondary_text));
    }

    body.spacing(scale::scaled(FOOT_GAP)).into()
}

/// Secondary text in the muted shade the theme provides.
fn secondary_text(theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(theme.extended_palette().background.weak.text),
        ..container::Style::default()
    }
}
