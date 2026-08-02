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

/// Builds the fill a card is drawn with.
///
/// Named rather than inlined so the paint of a card can be read back on its
/// own, the way the button styles are.
fn card_style(font_size: f32, opacity: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| container::Style {
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
    }
}

/// Renders a card the detail of a picked entry lives in.
pub fn card<'a, M: 'a>(content: Element<'a, M>, font_size: f32, opacity: f32) -> Element<'a, M> {
    container(content)
        .padding(style::card_padding(font_size))
        .width(Length::Fill)
        .style(card_style(font_size, opacity))
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
        .style(outline_style(font_size, opacity))
        .into()
}

/// Builds the border an outlined box is drawn with.
fn outline_style(font_size: f32, opacity: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| container::Style {
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
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Msg {
        Pressed
    }

    const FONT: f32 = 14.0;

    fn leaf<'a>() -> Element<'a, Msg> {
        text("leaf").into()
    }

    #[test]
    fn a_note_is_drawn_smaller_than_the_page_text() {
        let note: Element<'_, Msg> = note("nothing here yet", FONT);

        assert!(style::caption_size(FONT) < FONT);
        assert_eq!(
            note.as_widget().size(),
            iced::Size::new(Length::Shrink, Length::Shrink)
        );
    }

    #[test]
    fn a_section_fills_the_page_width() {
        let section: Element<'_, Msg> = section("Appearance", leaf(), FONT);

        assert_eq!(section.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn every_page_column_fills_the_width() {
        let page: Element<'_, Msg> = page(FONT).push(leaf()).into();
        let rows: Element<'_, Msg> = rows(FONT).push(leaf()).into();
        let grid: Element<'_, Msg> = grid(FONT).push(leaf()).into();

        for column in [page, rows, grid] {
            assert_eq!(column.as_widget().size().width, Length::Fill);
        }
    }

    #[test]
    fn the_page_column_is_spaced_wider_than_the_rows_it_holds() {
        assert!(style::section_gap(FONT) > style::page_gap(FONT));
        assert!(style::page_gap(FONT) > style::group_gap(FONT));
    }

    #[test]
    fn a_control_row_hugs_what_it_holds() {
        let controls: Element<'_, Msg> = controls(FONT).push(leaf()).into();
        let group: Element<'_, Msg> = group(FONT).push(leaf()).into();

        for row in [controls, group] {
            assert_eq!(row.as_widget().size().width, Length::Shrink);
        }
    }

    #[test]
    fn a_labelled_row_fills_the_width_and_reserves_the_label_column() {
        let row: Element<'_, Msg> = labelled_row("Style", leaf(), FONT);

        assert_eq!(row.as_widget().size().width, Length::Fill);
        assert!(style::label_width(FONT) > 0.0);
    }

    #[test]
    fn a_card_fills_the_width_it_is_given() {
        let card: Element<'_, Msg> = card(leaf(), FONT, 1.0);

        assert_eq!(card.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn an_outline_hugs_what_it_frames() {
        let outlined: Element<'_, Msg> = outlined(leaf(), FONT, 1.0);

        assert_eq!(
            outlined.as_widget().size(),
            iced::Size::new(Length::Shrink, Length::Shrink)
        );
    }

    #[test]
    fn a_card_and_an_outline_share_the_padding_and_the_corner() {
        assert!(style::corner_radius(FONT) > 0.0);
        const { assert!(style::BORDER_WIDTH > 0.0) };
        const { assert!(style::CARD_FILL_ALPHA > 0.0 && style::CARD_FILL_ALPHA <= 1.0) };

        let card = card_style(FONT, 1.0)(&Theme::Dark);
        let outline = outline_style(FONT, 1.0)(&Theme::Dark);

        assert_eq!(card.border.radius, outline.border.radius);
    }

    #[test]
    fn a_card_is_filled_by_a_faded_weak_background() {
        let theme = Theme::Dark;
        let filled = card_style(FONT, 1.0)(&theme);

        assert_eq!(
            filled.background,
            Some(Background::Color(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(style::CARD_FILL_ALPHA)
            ))
        );
        assert_eq!(filled.border.width, 0.0);
    }

    #[test]
    fn the_card_fill_follows_the_opacity_it_is_handed() {
        let opaque = card_style(FONT, 1.0)(&Theme::Dark).background;
        let faded = card_style(FONT, 0.25)(&Theme::Dark).background;

        let alpha = |background| match background {
            Some(Background::Color(color)) => color.a,
            _ => panic!("a card is always filled")
        };

        assert!(alpha(faded) < alpha(opaque));
    }

    #[test]
    fn an_outline_is_bordered_and_never_filled() {
        let theme = Theme::Dark;
        let outline = outline_style(FONT, 1.0)(&theme);

        assert!(outline.background.is_none());
        assert_eq!(outline.border.width, style::BORDER_WIDTH);
        assert_eq!(
            outline.border.color,
            theme.extended_palette().secondary.strong.color
        );
    }

    #[test]
    fn the_outline_fades_with_the_opacity_it_is_handed() {
        let faded = outline_style(FONT, 0.5)(&Theme::Dark);

        assert!(faded.border.color.a < outline_style(FONT, 1.0)(&Theme::Dark).border.color.a);
    }

    #[test]
    fn a_wider_page_text_rounds_the_card_further() {
        assert_ne!(
            card_style(24.0, 1.0)(&Theme::Dark).border.radius,
            card_style(10.0, 1.0)(&Theme::Dark).border.radius
        );
    }

    #[test]
    fn a_scaffolded_page_carries_the_message_of_what_it_holds() {
        let pressed: Element<'_, Msg> = iced::widget::button(text("press"))
            .on_press(Msg::Pressed)
            .into();
        let page: Element<'_, Msg> = page(FONT)
            .push(section("Modules", card(pressed, FONT, 1.0), FONT))
            .into();

        assert_eq!(page.as_widget().size().width, Length::Fill);
    }
}
