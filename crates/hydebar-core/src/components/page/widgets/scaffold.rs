//! The frames a page is built from: columns, rows, sections and cards.
//!
//! Handed out ready-spaced and ready-padded so a page states what it holds
//! and never how far apart it holds it.

use iced::{
    Alignment, Background, Border, Element, Length, Theme,
    widget::{Column, Row, container, text}
};

use crate::components::page::style;

/// Renders a note: a sentence a page states in place of a control it cannot
/// offer, such as a section that holds nothing yet.
///
/// Drawn smaller than a value so it reads as an aside rather than as something
/// the bar is reporting.
pub fn note<'a, M: 'a>(label: impl text::IntoFragment<'a>, font_size: f32) -> Element<'a, M> {
    text(label).size(style::caption_size(font_size)).into()
}

/// Renders the heading of a section.
///
/// Every heading on every tab comes from here, so no page can invent a heading
/// of its own size or weight.
fn section_title<'a, M: 'a>(label: impl text::IntoFragment<'a>, font_size: f32) -> Element<'a, M> {
    text(label)
        .size(style::section_title_size(font_size))
        .into()
}

/// Renders a titled section: the heading, then what it holds.
///
/// A page is a stack of these and nothing else, which is what makes the three
/// tabs read as one window.
pub fn section<'a, M: 'a>(
    title: impl text::IntoFragment<'a>,
    content: Element<'a, M>,
    font_size: f32
) -> Element<'a, M> {
    Column::new()
        .push(section_title(title, font_size))
        .push(content)
        .spacing(style::caption_gap(font_size))
        .width(Length::Fill)
        .into()
}

/// Starts the column a page is stacked in.
///
/// Handed out ready-spaced and ready-padded so a page states what it holds and
/// never how far apart it holds it.
pub fn page<'a, M: 'a>(font_size: f32) -> Column<'a, M> {
    Column::new()
        .spacing(style::section_gap(font_size))
        .padding(style::page_padding(font_size))
        .width(Length::Fill)
}

/// Starts the column the rows of one section are stacked in.
pub fn rows<'a, M: 'a>(font_size: f32) -> Column<'a, M> {
    Column::new()
        .spacing(style::page_gap(font_size))
        .width(Length::Fill)
}

/// Starts the column a wrapping grid of chips is stacked in.
///
/// Chips sit closer together than rows do, in both directions, so a grid reads
/// as one block rather than as a stack of unrelated rows.
pub fn grid<'a, M: 'a>(font_size: f32) -> Column<'a, M> {
    Column::new()
        .spacing(style::group_gap(font_size))
        .width(Length::Fill)
}

/// Starts the row the controls of one setting sit in.
pub fn controls<'a, M: 'a>(font_size: f32) -> Row<'a, M> {
    Row::new()
        .spacing(style::row_gap(font_size))
        .align_y(Alignment::Center)
}

/// Starts the row items that belong together sit in, such as the chips of an
/// island or the buttons of one action.
pub fn group<'a, M: 'a>(font_size: f32) -> Row<'a, M> {
    Row::new()
        .spacing(style::group_gap(font_size))
        .align_y(Alignment::Center)
}

/// Renders a labelled row whose label is not known until it is drawn, such as
/// the number of an island.
///
/// Shares the label column with every other row on every other tab, so the
/// islands of the module page line up with the steppers of the appearance page.
pub fn labelled_row<'a, M: 'a>(
    label: impl text::IntoFragment<'a>,
    content: Element<'a, M>,
    font_size: f32
) -> Element<'a, M> {
    Row::new()
        .push(
            text(label)
                .size(font_size)
                .width(Length::Fixed(style::label_width(font_size)))
        )
        .push(content)
        .spacing(style::row_gap(font_size))
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

/// Renders a card the detail of a picked entry lives in.
pub fn card<'a, M: 'a>(content: Element<'a, M>, font_size: f32, opacity: f32) -> Element<'a, M> {
    container(content)
        .padding(style::card_padding(font_size))
        .width(Length::Fill)
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity * style::CARD_FILL_ALPHA)
            )),
            border: Border::default().rounded(style::corner_radius(font_size)),
            ..container::Style::default()
        })
        .into()
}

/// Renders an outlined box around `content`.
///
/// The outline carries the same padding and the same corner as a card, so an
/// island on the module page sits on the same grid as the card below it.
pub fn outlined<'a, M: 'a>(
    content: Element<'a, M>,
    font_size: f32,
    opacity: f32
) -> Element<'a, M> {
    container(content)
        .padding(style::card_padding(font_size))
        .style(move |theme: &Theme| container::Style {
            border: Border {
                width:  style::BORDER_WIDTH,
                radius: style::corner_radius(font_size).into(),
                color:  theme
                    .extended_palette()
                    .secondary
                    .strong
                    .color
                    .scale_alpha(opacity)
            },
            ..container::Style::default()
        })
        .into()
}
