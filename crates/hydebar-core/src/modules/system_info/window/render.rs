//! Drawing of the window: sections, aligned value columns and themed
//! usage meters.

use iced::{
    Alignment, Border, Color, Element, Length, Theme,
    widget::{Column, Row as BarRow, Space, column, container, row, rule}
};

use super::{
    super::{Message, data::SystemInfoData},
    metrics::{
        self, FOOT_GAP, METER_GAP, METER_HEIGHT, NOTE_SIZE, OUTER_PADDING, ROW_GAP,
        ROW_SIZE, SECTION_GAP, SECTION_TITLE_SIZE, TITLE_SIZE
    },
    model::{self, MeterLevel, Row, Section}
};
use crate::{
    components::{
        icons::{IconTheme, icon},
        scale,
        text::text
    },
    config::SystemModuleConfig
};

/// Render the module menu displaying detailed system metrics.
#[must_use]
pub fn build_menu_view<'a>(
    data: &'a SystemInfoData,
    config: &SystemModuleConfig,
    icons: &IconTheme
) -> Element<'a, Message> {
    let sections = model::sections(data);
    let footnotes = model::footnotes(data, config);

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
        body = body.push(
            container(text(note).size(scale::scaled(NOTE_SIZE))).style(secondary_text)
        );
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
            .spacing(scale::scaled(METER_GAP))
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

/// A usage meter: the filled share over a weak track, coloured by how
/// full the pool is.
fn meter_view<'a>(percent: u32) -> Element<'a, Message> {
    let percent = percent.min(100);
    let radius = scale::scaled(METER_HEIGHT / 2.0);

    let mut bar = BarRow::new();

    if percent > 0 {
        bar = bar.push(
            container(Space::new().width(Length::Fill).height(Length::Fill))
                .width(Length::FillPortion(percent as u16))
                .height(Length::Fill)
                .style(move |theme: &Theme| fill_style(theme, percent, radius))
        );
    }

    if percent < 100 {
        bar = bar.push(Space::new().width(Length::FillPortion((100 - percent) as u16)));
    }

    container(bar)
        .width(Length::Fill)
        .height(Length::Fixed(scale::scaled(METER_HEIGHT)))
        .style(move |theme: &Theme| track_style(theme, radius))
        .into()
}

fn footnotes_view<'a>(footnotes: Vec<String>) -> Element<'a, Message> {
    let mut body = Column::new().push(rule::horizontal(1)).push(
        container(
            text("Not reported on this machine").size(scale::scaled(SECTION_TITLE_SIZE))
        )
        .style(secondary_text)
    );

    for entry in footnotes {
        body = body.push(
            container(text(entry).size(scale::scaled(NOTE_SIZE))).style(secondary_text)
        );
    }

    body.spacing(scale::scaled(FOOT_GAP)).into()
}

fn fill_style(theme: &Theme, percent: u32, radius: f32) -> container::Style {
    let colour = match model::meter_level(percent) {
        MeterLevel::Calm => theme.palette().primary,
        MeterLevel::Busy => theme.palette().warning,
        MeterLevel::Critical => theme.palette().danger
    };

    rounded(colour.into(), radius)
}

fn track_style(theme: &Theme, radius: f32) -> container::Style {
    rounded(
        theme.extended_palette().background.weak.color.into(),
        radius
    )
}

fn rounded(background: iced::Background, radius: f32) -> container::Style {
    container::Style {
        background: Some(background),
        border: Border {
            width:  0.0,
            radius: radius.into(),
            color:  Color::TRANSPARENT
        },
        ..container::Style::default()
    }
}

/// Secondary text in the muted shade the theme provides.
fn secondary_text(theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(theme.extended_palette().background.weak.text),
        ..container::Style::default()
    }
}
