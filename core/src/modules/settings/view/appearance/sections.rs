//! Rows of each section of the appearance page.

use iced::Element;

use super::{HYDE_BRANCH, NOTIFICATIONS};
use crate::{
    components::{
        page::widgets::{choice_row, rows as row_stack, stepper_row},
        push_maybe::PushMaybe
    },
    config::{
        Appearance, AppearanceStyle, BarLayer, Config, HydeBranch, NotificationSource, Position
    },
    modules::settings::{Message, Settings}
};

/// Height the bar falls back to while the configuration names none.
const FALLBACK_HEIGHT: f32 = 34.0;

/// Rows of the placement section, against the running `config`.
pub(super) fn placement_rows(config: &Config, font_size: f32, opacity: f32) -> Element<'_, Message> {
    row_stack(font_size)
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
        .into()
}

/// Rows of the size section, with the sizes as the file spells them.
pub(super) fn size_rows(
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

/// Rows of the desktop section, against the running `config`.
pub(super) fn desktop_rows(config: &Config, font_size: f32, opacity: f32) -> Element<'_, Message> {
    row_stack(font_size)
        .push(choice_row(
            NOTIFICATIONS,
            NotificationSource::ALL
                .into_iter()
                .map(|source| {
                    (
                        source.label(),
                        source,
                        config.notifications.source == source
                    )
                })
                .collect(),
            Message::SetNotificationSource,
            font_size,
            opacity
        ))
        .push_maybe(config.updates.as_ref().map(|updates| {
            choice_row(
                HYDE_BRANCH,
                HydeBranch::ALL
                    .into_iter()
                    .map(|branch| (branch.label(), branch, updates.hyde_branch == branch))
                    .collect(),
                Message::SetHydeBranch,
                font_size,
                opacity
            )
        }))
        .into()
}
