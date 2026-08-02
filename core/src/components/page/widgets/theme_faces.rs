//! The two faces a theme card wears: a line in a list, or a tile in a
//! grid. Both read their colours through the same paint so a card looks
//! the same whichever way the page lays it.

use std::path::PathBuf;

use iced::{
    Alignment, Element, Length, Theme,
    widget::{Column, Row, button, container, text}
};

use super::theme_card::{ChipPaint, DOT_GAP_EM, ThemeChip, busy_strip, card_colors, palette_dots};
use crate::components::icons::icon_raw_sized;

/// The paint a face presses in: unfilled, so only the card behind it carries
/// a surface, and inked with the colour the card's own state settles on.
fn face_style(
    paint_colors: Option<(iced::Color, iced::Color, iced::Color)>,
    state: ThemeChip
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, _| button::Style {
        background: None,
        text_color: card_colors(theme, paint_colors, state).1,
        ..button::Style::default()
    }
}

/// The screenshot of a theme, sized to the room its face gives it.
fn preview(path: &PathBuf, height: f32) -> iced::widget::Image<iced::widget::image::Handle> {
    iced::widget::image(iced::widget::image::Handle::from_path(path))
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .content_fit(iced::ContentFit::Cover)
}

/// The face of a card drawn as one line of a list: name, thumbnail and
/// palette side by side, deeds at the end, the busy strip underneath.
#[expect(
    clippy::too_many_arguments,
    reason = "the face states everything one card line shows in one call"
)]
pub(super) fn horizontal_face<'a, M: Clone + 'static>(
    label: String,
    badge: Option<&'static str>,
    screenshot: Option<&PathBuf>,
    paint: Option<&ChipPaint>,
    paint_colors: Option<(iced::Color, iced::Color, iced::Color)>,
    state: ThemeChip,
    control: f32,
    deeds: Option<Row<'a, M>>
) -> Element<'a, M> {
    let mut name_row = Row::new()
        .spacing(DOT_GAP_EM * control * 0.5)
        .align_y(Alignment::Center)
        .push(text(label).size(control));

    if let Some(glyph) = badge {
        name_row = name_row.push(icon_raw_sized(glyph.to_owned(), Some(control * 0.8)));
    }

    let mut face = Row::new()
        .spacing(DOT_GAP_EM * control)
        .align_y(Alignment::Center)
        .push(container(name_row).width(Length::Fill))
        .width(Length::Fill);

    if let Some(thumb) = screenshot.map(|path| preview(path, control * 2.2)) {
        face = face.push(container(thumb).width(Length::Fixed(control * 4.0)));
    }

    if let Some(paint) = paint {
        face = face.push(
            container(palette_dots::<M>(paint.palette.clone(), control)).width(Length::Fill)
        );
    }

    let pressable = button(face)
        .padding(0)
        .style(face_style(paint_colors, state))
        .width(Length::Fill);

    let mut line = Row::new()
        .align_y(Alignment::Center)
        .spacing(DOT_GAP_EM * control)
        .push(pressable);

    if let Some(deeds) = deeds {
        line = line.push(deeds);
    }

    let mut column = Column::new().push(line).spacing(DOT_GAP_EM * control);

    if let ThemeChip::Applying(spinner) = state {
        column = column.push(busy_strip(spinner, control));
    }

    column.into()
}

/// The face of a card drawn as a tile of the grid: screenshot, name and
/// palette stacked, deeds under the tile, the busy strip in the stack.
#[expect(
    clippy::too_many_arguments,
    reason = "the face states everything one card tile shows in one call"
)]
pub(super) fn vertical_face<'a, M: Clone + 'static>(
    label: String,
    badge: Option<&'static str>,
    screenshot: Option<&PathBuf>,
    paint: Option<&ChipPaint>,
    paint_colors: Option<(iced::Color, iced::Color, iced::Color)>,
    state: ThemeChip,
    control: f32,
    cell: f32,
    deeds: Option<Row<'a, M>>
) -> Element<'a, M> {
    let name: Element<'a, M> = if let Some(glyph) = badge {
        container(
            Row::new()
                .spacing(DOT_GAP_EM * control * 0.5)
                .align_y(Alignment::Center)
                .push(text(label).size(control))
                .push(icon_raw_sized(glyph.to_owned(), Some(control * 0.8)))
        )
        .width(iced::Length::Fill)
        .align_x(iced::Alignment::Center)
        .into()
    } else {
        text(label)
            .size(control)
            .width(iced::Length::Fill)
            .align_x(iced::Alignment::Center)
            .into()
    };

    let body: Element<'a, M> = match paint {
        Some(paint) => {
            let mut column = Column::new().spacing(DOT_GAP_EM * control);

            if let Some(shot) = screenshot.map(|path| preview(path, cell * 0.5)) {
                column = column.push(shot);
            }

            let mut column = column
                .push(name)
                .push(palette_dots(paint.palette.clone(), control));

            if let ThemeChip::Applying(spinner) = state {
                column = column.push(busy_strip(spinner, control));
            }

            column.into()
        }
        None => name
    };

    let pressable = button(container(body).width(Length::Fill))
        .padding(0)
        .style(face_style(paint_colors, state))
        .width(Length::Fill);

    let mut column = Column::new()
        .push(pressable)
        .spacing(DOT_GAP_EM * control)
        .align_x(Alignment::Center);

    if let Some(deeds) = deeds {
        column = column.push(deeds);
    }

    column.into()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use iced::Color;

    use super::*;
    use crate::modules::themes::Spinner;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Msg {
        Deed
    }

    const CONTROL: f32 = 12.0;
    const COLORS: (Color, Color, Color) = (
        Color::from_rgb(0.1, 0.1, 0.1),
        Color::from_rgb(0.9, 0.9, 0.9),
        Color::from_rgb(0.5, 0.0, 0.5)
    );

    fn paint() -> ChipPaint {
        ChipPaint {
            background: COLORS.0,
            text:       COLORS.1,
            accent:     COLORS.2,
            palette:    vec![Color::WHITE, Color::BLACK]
        }
    }

    fn shot() -> PathBuf {
        PathBuf::from("/nonexistent/theme.png")
    }

    fn deeds<'a>() -> Row<'a, Msg> {
        Row::new().push(button(text("x")).on_press(Msg::Deed))
    }

    #[test]
    fn a_bare_line_shows_only_its_name() {
        let face: Element<'_, Msg> = horizontal_face(
            "Mocha".to_owned(),
            None,
            None,
            None,
            None,
            ThemeChip::Idle,
            CONTROL,
            None
        );

        assert_eq!(face.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_full_line_carries_the_badge_the_shot_the_palette_and_the_deeds() {
        let face: Element<'_, Msg> = horizontal_face(
            "Mocha".to_owned(),
            Some("\u{f00c}"),
            Some(&shot()),
            Some(&paint()),
            Some(COLORS),
            ThemeChip::Active,
            CONTROL,
            Some(deeds())
        );

        assert_eq!(face.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_line_being_applied_grows_a_busy_strip() {
        let face: Element<'_, Msg> = horizontal_face(
            "Mocha".to_owned(),
            None,
            None,
            Some(&paint()),
            Some(COLORS),
            ThemeChip::Applying(Spinner::default()),
            CONTROL,
            None
        );

        assert_eq!(face.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_bare_tile_shows_only_its_name() {
        let face: Element<'_, Msg> = vertical_face(
            "Mocha".to_owned(),
            None,
            None,
            None,
            None,
            ThemeChip::Idle,
            CONTROL,
            120.0,
            None
        );

        assert_eq!(face.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_badged_tile_centres_the_name_beside_its_glyph() {
        let face: Element<'_, Msg> = vertical_face(
            "Mocha".to_owned(),
            Some("\u{f00c}"),
            None,
            None,
            None,
            ThemeChip::Inert,
            CONTROL,
            120.0,
            None
        );

        assert_eq!(face.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_full_tile_stacks_the_shot_the_name_the_palette_and_the_deeds() {
        let face: Element<'_, Msg> = vertical_face(
            "Mocha".to_owned(),
            Some("\u{f00c}"),
            Some(&shot()),
            Some(&paint()),
            Some(COLORS),
            ThemeChip::Applying(Spinner::default()),
            CONTROL,
            120.0,
            Some(deeds())
        );

        assert_eq!(face.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_painted_tile_without_a_shot_still_draws_its_palette() {
        let face: Element<'_, Msg> = vertical_face(
            "Mocha".to_owned(),
            None,
            None,
            Some(&paint()),
            Some(COLORS),
            ThemeChip::Idle,
            CONTROL,
            120.0,
            None
        );

        assert_eq!(face.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_face_presses_without_a_surface_of_its_own() {
        let theme = Theme::Dark;
        let styled = face_style(Some(COLORS), ThemeChip::Active)(&theme, button::Status::Active);

        assert!(styled.background.is_none());
        assert_eq!(
            styled.text_color,
            card_colors(&theme, Some(COLORS), ThemeChip::Active).1
        );
    }

    #[test]
    fn a_blocked_face_is_inked_more_faintly_than_an_idle_one() {
        let theme = Theme::Dark;
        let idle = face_style(Some(COLORS), ThemeChip::Idle)(&theme, button::Status::Active);
        let blocked = face_style(Some(COLORS), ThemeChip::Blocked)(&theme, button::Status::Active);

        assert!(blocked.text_color.a < idle.text_color.a);
    }
}
