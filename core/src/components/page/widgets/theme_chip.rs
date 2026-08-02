//! The card of the theme grid: the one widget on the page whose press
//! starts something the bar cannot take back or hurry.

use iced::{
    Alignment, Background, Border, Element, Theme,
    widget::{Row, button, container}
};

use super::{
    theme_card::{ChipPaint, DOT_GAP_EM, ThemeChip, card_colors, card_ring},
    theme_faces::{horizontal_face, vertical_face}
};
use crate::components::{icons::icon_raw_sized, page::style};

/// Renders a chip of the theme grid.
///
/// Kept apart from [`super::chip`], which draws a picture of the bar and is
/// never blocked, because this one has to say three things at once: which
/// theme the desktop is on, which one it is moving to, and that nothing else
/// can be asked for until it gets there.
///
/// Given a `paint`, the chip is drawn in the colours of the theme it stands
/// for — the grid becomes a palette of the themes themselves — and the theme
/// in force is told apart by a ring of its own accent, since a fill can no
/// longer mark it.
#[expect(
    clippy::too_many_arguments,
    reason = "the chip states everything a theme tile shows in one call"
)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "callers outside this module hand over owned paint and screenshot values"
)]
pub fn theme_chip<'a, M: Clone + 'static>(
    label: String,
    badge: Option<&'static str>,
    message: M,
    state: ThemeChip,
    font_size: f32,
    opacity: f32,
    cell: f32,
    paint: Option<ChipPaint>,
    screenshot: Option<std::path::PathBuf>,
    actions: Vec<(&'static str, M, bool)>,
    horizontal: bool
) -> Element<'a, M> {
    let control = style::control_size(font_size);

    let paint_colors = paint
        .as_ref()
        .map(|paint| (paint.background, paint.text, paint.accent));

    let deeds = deeds_row(actions, control, opacity);

    let press = state.is_pressable().then_some(message);

    let content: Element<'a, M> = if horizontal {
        horizontal_face(
            label,
            badge,
            screenshot.as_ref(),
            paint.as_ref(),
            paint_colors,
            state,
            control,
            deeds
        )
    } else {
        vertical_face(
            label,
            badge,
            screenshot.as_ref(),
            paint.as_ref(),
            paint_colors,
            state,
            control,
            cell,
            deeds
        )
    };

    let card = container(content).width(cell).padding([
        style::CHIP_PADDING_EM[0] * control,
        style::CHIP_PADDING_EM[1] * control
    ]);

    let card = card.style(card_style(paint_colors, state, font_size, opacity));

    match press {
        Some(message) => iced::widget::mouse_area(card).on_press(message).into(),
        None => card.into()
    }
}

/// The surface of a theme card, ringed when its state calls for one.
fn card_style(
    paint_colors: Option<(iced::Color, iced::Color, iced::Color)>,
    state: ThemeChip,
    font_size: f32,
    opacity: f32
) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| {
        let (background, text_color, ringed) = card_colors(theme, paint_colors, state);

        let mut border = Border::default().rounded(style::corner_radius(font_size));
        if ringed {
            let ring = card_ring(theme, paint_colors, state);

            border = border.color(ring).width(2.0);
        }

        container::Style {
            background: Some(Background::Color(background.scale_alpha(opacity))),
            text_color: Some(text_color),
            border,
            ..container::Style::default()
        }
    }
}

/// The row of per-card actions, each a small glyph answering its own press.
///
/// An action that cannot run right now is drawn without a press handler, so
/// the refusal is something the pointer meets rather than a press the module
/// has to swallow.
fn deeds_row<'a, M: Clone + 'static>(
    actions: Vec<(&'static str, M, bool)>,
    control: f32,
    opacity: f32
) -> Option<Row<'a, M>> {
    if actions.is_empty() {
        return None;
    }

    let mut row = Row::new()
        .spacing(DOT_GAP_EM * control)
        .align_y(Alignment::Center);

    for (glyph, deed, enabled) in actions {
        let deed_button = button(icon_raw_sized(glyph.to_owned(), Some(control * 0.9)))
            .padding(control * 0.15)
            .style(crate::style::ghost_button_style(opacity));

        row = row.push(if enabled {
            Element::from(iced::widget::mouse_area(deed_button).on_press(deed))
        } else {
            deed_button.into()
        });
    }

    Some(row)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::{Color, Length};

    use super::*;
    use crate::modules::themes::Spinner;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Msg {
        Apply,
        Remove
    }

    const FONT: f32 = 14.0;
    const CELL: f32 = 120.0;
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

    fn card(state: ThemeChip, horizontal: bool) -> Element<'static, Msg> {
        theme_chip(
            "Mocha".to_owned(),
            Some("\u{f00c}"),
            Msg::Apply,
            state,
            FONT,
            1.0,
            CELL,
            Some(paint()),
            Some(std::path::PathBuf::from("/nonexistent/theme.png")),
            vec![("\u{f1f8}", Msg::Remove, true)],
            horizontal
        )
    }

    #[test]
    fn a_chip_takes_the_cell_width_it_is_given() {
        assert_eq!(
            card(ThemeChip::Idle, false).as_widget().size().width,
            Length::Fixed(CELL)
        );
    }

    #[test]
    fn a_chip_is_drawn_the_same_width_in_either_layout() {
        assert_eq!(
            card(ThemeChip::Idle, true).as_widget().size().width,
            card(ThemeChip::Idle, false).as_widget().size().width
        );
    }

    #[test]
    fn every_state_of_a_chip_can_be_drawn() {
        for state in [
            ThemeChip::Active,
            ThemeChip::Idle,
            ThemeChip::Applying(Spinner::default()),
            ThemeChip::Blocked,
            ThemeChip::Condemned,
            ThemeChip::Inert
        ] {
            assert_eq!(
                card(state, false).as_widget().size().width,
                Length::Fixed(CELL)
            );
        }
    }

    #[test]
    fn an_unpainted_chip_without_deeds_is_still_a_card() {
        let bare: Element<'_, Msg> = theme_chip(
            "Mocha".to_owned(),
            None,
            Msg::Apply,
            ThemeChip::Idle,
            FONT,
            1.0,
            CELL,
            None,
            None,
            Vec::new(),
            false
        );

        assert_eq!(bare.as_widget().size().width, Length::Fixed(CELL));
    }

    #[test]
    fn a_ringed_state_draws_a_border_an_idle_one_does_not() {
        let theme = Theme::Dark;
        let idle = card_style(Some(COLORS), ThemeChip::Idle, FONT, 1.0)(&theme);
        let active = card_style(Some(COLORS), ThemeChip::Active, FONT, 1.0)(&theme);

        assert_eq!(idle.border.width, 0.0);
        assert_eq!(active.border.width, 2.0);
        assert_eq!(active.border.color, card_ring(&theme, Some(COLORS), ThemeChip::Active));
    }

    #[test]
    fn a_condemned_card_is_ringed_in_danger() {
        let theme = Theme::Dark;
        let condemned = card_style(Some(COLORS), ThemeChip::Condemned, FONT, 1.0)(&theme);

        assert_eq!(condemned.border.color, theme.palette().danger);
    }

    #[test]
    fn a_card_is_filled_with_the_colours_its_state_settles_on() {
        let theme = Theme::Dark;
        let styled = card_style(Some(COLORS), ThemeChip::Idle, FONT, 1.0)(&theme);
        let (background, text_color, _) = card_colors(&theme, Some(COLORS), ThemeChip::Idle);

        assert_eq!(styled.background, Some(Background::Color(background)));
        assert_eq!(styled.text_color, Some(text_color));
    }

    #[test]
    fn the_card_fill_fades_with_the_opacity_it_is_handed() {
        let theme = Theme::Dark;
        let alpha = |opacity| match card_style(Some(COLORS), ThemeChip::Idle, FONT, opacity)(&theme)
            .background
        {
            Some(Background::Color(color)) => color.a,
            _ => panic!("a card is always filled")
        };

        assert!(alpha(0.4) < alpha(1.0));
    }

    #[test]
    fn a_deedless_card_grows_no_action_row() {
        assert!(deeds_row::<Msg>(Vec::new(), 12.0, 1.0).is_none());
    }

    #[test]
    fn a_deed_that_cannot_run_is_drawn_without_a_press() {
        let row = deeds_row(
            vec![("\u{f1f8}", Msg::Remove, false), ("\u{f00c}", Msg::Apply, true)],
            12.0,
            1.0
        );

        assert!(row.is_some());
        let row: Element<'_, Msg> = row.expect("two deeds make a row").into();
        assert_eq!(row.as_widget().size().width, Length::Shrink);
    }
}
