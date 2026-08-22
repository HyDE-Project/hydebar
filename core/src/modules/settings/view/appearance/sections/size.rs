//! How big the bar is drawn: its height, its text and its magnification.

use iced::Element;

use crate::{
    components::{
        page::widgets::{choice_row, rows as row_stack, stepper_row},
        push_maybe::PushMaybe
    },
    config::{Appearance, AppearanceStyle, Config},
    modules::settings::{Message, Settings}
};

/// Height the bar falls back to while the configuration names none.
pub(super) const FALLBACK_HEIGHT: f32 = 34.0;

/// Rows of the size section, with the sizes as the file spells them.
pub fn size_rows(
    config: &Config,
    magnification: f32,
    font_size: f32,
    opacity: f32
) -> Element<'_, Message> {
    let appearance: &Appearance = &config.appearance;
    let written_font_size = font_size / magnification;
    let height = appearance.height.unwrap_or(FALLBACK_HEIGHT) / magnification;
    let side_padding = appearance.bar_padding()[1] / magnification;

    row_stack(font_size)
        .push(choice_row(
            "Style",
            vec![
                (
                    "Islands",
                    AppearanceStyle::Islands,
                    appearance.style == AppearanceStyle::Islands
                ),
                (
                    "Solid",
                    AppearanceStyle::Solid,
                    appearance.style == AppearanceStyle::Solid
                ),
                (
                    "Gradient",
                    AppearanceStyle::Gradient,
                    appearance.style == AppearanceStyle::Gradient
                ),
            ],
            Message::SetStyle,
            font_size,
            opacity
        ))
        .push_maybe((!appearance.auto_scale).then(|| {
            stepper_row(
                "Height",
                format!("{height:.0}"),
                Message::SetHeight(Settings::height_below(height)),
                Message::SetHeight(Settings::height_above(height)),
                font_size,
                opacity
            )
        }))
        .push_maybe((!appearance.auto_scale).then(|| {
            stepper_row(
                "Side padding",
                format!("{side_padding:.0}"),
                Message::SetSidePadding(Settings::side_padding_below(side_padding)),
                Message::SetSidePadding(Settings::side_padding_above(side_padding)),
                font_size,
                opacity
            )
        }))
        .push_maybe((!appearance.auto_scale).then(|| {
            stepper_row(
                "Font size",
                format!("{written_font_size:.0}"),
                Message::SetFontSize(Settings::font_size_below(written_font_size)),
                Message::SetFontSize(Settings::font_size_above(written_font_size)),
                font_size,
                opacity
            )
        }))
        .push(stepper_row(
            "Opacity",
            format!("{:.2}", appearance.opacity),
            Message::SetOpacity(Settings::opacity_below(appearance.opacity)),
            Message::SetOpacity(Settings::opacity_above(appearance.opacity)),
            font_size,
            opacity
        ))
        .into()
}
