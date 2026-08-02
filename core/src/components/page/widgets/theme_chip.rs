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

    let card = card.style(move |theme: &Theme| {
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
    });

    match press {
        Some(message) => iced::widget::mouse_area(card).on_press(message).into(),
        None => card.into()
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
