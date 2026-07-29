//! Appearance page of the settings window.

use iced::{Element, Length, widget::Column};

use super::widgets::{choice_row, stepper_row};
use crate::{
    config::{Appearance, AppearanceStyle, BarLayer, Config, DEFAULT_FONT_SIZE, Position},
    modules::settings::{Message, Settings}
};

/// Height the bar falls back to while the configuration names none.
const FALLBACK_HEIGHT: f32 = 34.0;

/// Gap between the rows of the page, in multiples of the text size.
const PAGE_GAP_EM: f32 = 1.2;

/// Renders the appearance page against the running `config`.
pub(super) fn view(config: &Config, opacity: f32) -> Element<'_, Message> {
    let appearance: &Appearance = &config.appearance;
    let font_size = appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
    let height = appearance.height.unwrap_or(FALLBACK_HEIGHT);

    Column::new()
        .push(choice_row(
            "Position",
            vec![
                ("Top", Position::Top, config.position == Position::Top),
                (
                    "Bottom",
                    Position::Bottom,
                    config.position == Position::Bottom
                ),
            ],
            Message::SetPosition,
            font_size,
            opacity
        ))
        .push(choice_row(
            "Layer",
            vec![
                ("Bottom", BarLayer::Bottom, config.layer == BarLayer::Bottom),
                ("Top", BarLayer::Top, config.layer == BarLayer::Top),
                (
                    "Overlay",
                    BarLayer::Overlay,
                    config.layer == BarLayer::Overlay
                ),
            ],
            Message::SetLayer,
            font_size,
            opacity
        ))
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
        .push(stepper_row(
            "Height",
            format!("{height:.0}"),
            Message::SetHeight(Settings::height_below(height)),
            Message::SetHeight(Settings::height_above(height)),
            font_size,
            opacity
        ))
        .push(stepper_row(
            "Font size",
            format!("{font_size:.0}"),
            Message::SetFontSize(Settings::font_size_below(font_size)),
            Message::SetFontSize(Settings::font_size_above(font_size)),
            font_size,
            opacity
        ))
        .push(stepper_row(
            "Opacity",
            format!("{:.2}", appearance.opacity),
            Message::SetOpacity(Settings::opacity_below(appearance.opacity)),
            Message::SetOpacity(Settings::opacity_above(appearance.opacity)),
            font_size,
            opacity
        ))
        .push(choice_row(
            "Follow HyDE theme",
            vec![
                ("On", true, appearance.follow_hyde),
                ("Off", false, !appearance.follow_hyde),
            ],
            Message::SetFollowHyde,
            font_size,
            opacity
        ))
        .width(Length::Fill)
        .spacing(PAGE_GAP_EM * font_size)
        .into()
}
