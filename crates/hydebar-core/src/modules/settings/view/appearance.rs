//! Appearance page of the settings window.

use iced::{Element, Length, widget::Column};

use super::{
    metrics::{ROW_HEIGHT_EM, button_row_width, text_width},
    widgets::{ROW_GAP_EM, choice_row, stepper_row}
};
use crate::{
    config::{
        Appearance, AppearanceStyle, BarLayer, Config, DEFAULT_FONT_SIZE, NotificationSource,
        Position
    },
    modules::settings::{Message, Settings}
};

/// Height the bar falls back to while the configuration names none.
const FALLBACK_HEIGHT: f32 = 34.0;

/// Gap between the rows of the page, in multiples of the text size.
const PAGE_GAP_EM: f32 = 1.2;

/// Renders the appearance page against the running `config`.
///
/// Sizes are shown as they are written in the file, not as the bar draws them:
/// the window magnifies what it renders, and a stepper that showed the
/// magnified size would write it back and magnify it a second time.
///
/// The side padding is shown as the one in force rather than as the one the
/// file names, since a file that names none leaves the bar following the window
/// gaps of the compositor: stepping from the gap actually drawn is what makes
/// the first press nudge the bar instead of jumping it.
pub(super) fn view(config: &Config, opacity: f32, magnification: f32) -> Element<'_, Message> {
    let appearance: &Appearance = &config.appearance;
    let magnification = if magnification > 0.0 {
        magnification
    } else {
        1.0
    };
    let font_size = appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
    let written_font_size = font_size / magnification;
    let height = appearance.height.unwrap_or(FALLBACK_HEIGHT) / magnification;
    let side_padding = appearance.bar_padding()[1] / magnification;

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
            "Side padding",
            format!("{side_padding:.0}"),
            Message::SetSidePadding(Settings::side_padding_below(side_padding)),
            Message::SetSidePadding(Settings::side_padding_above(side_padding)),
            font_size,
            opacity
        ))
        .push(stepper_row(
            "Font size",
            format!("{written_font_size:.0}"),
            Message::SetFontSize(Settings::font_size_below(written_font_size)),
            Message::SetFontSize(Settings::font_size_above(written_font_size)),
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
            "Scale to the screen",
            vec![
                ("On", true, appearance.auto_scale),
                ("Off", false, !appearance.auto_scale),
            ],
            Message::SetAutoScale,
            font_size,
            opacity
        ))
        .push(choice_row(
            "Notifications",
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

/// Longest row of this page, which is how wide the window has to be.
///
/// The notification row is measured from the choices themselves rather than
/// from a copy of their names: a fourth source, or a renamed one, would
/// otherwise be cut off by a window sized for the old list.
#[must_use]
pub(super) fn desired_width(font_size: f32) -> f32 {
    let gap = ROW_GAP_EM * font_size;

    let rows: [(&str, &[&str]); 9] = [
        ("Position", &["Top", "Bottom"]),
        ("Layer", &["Bottom", "Top", "Overlay"]),
        ("Style", &["Islands", "Solid", "Gradient"]),
        ("Height", &["\u{2212}", "000", "+"]),
        ("Side padding", &["\u{2212}", "000", "+"]),
        ("Font size", &["\u{2212}", "000", "+"]),
        ("Opacity", &["\u{2212}", "0.00", "+"]),
        ("Follow HyDE theme", &["On", "Off"]),
        ("Scale to the screen", &["On", "Off"])
    ];

    let notifications = text_width("Notifications", font_size)
        + gap
        + button_row_width(
            NotificationSource::ALL
                .into_iter()
                .map(|source| source.label()),
            font_size,
            gap
        );

    rows.into_iter()
        .map(|(label, controls)| {
            text_width(label, font_size)
                + gap
                + button_row_width(controls.iter().copied(), font_size, gap)
        })
        .fold(notifications, f32::max)
}

/// Height this page needs.
pub(super) fn desired_height(font_size: f32) -> f32 {
    const ROWS: f32 = 10.0;

    ROWS * (ROW_HEIGHT_EM * font_size + PAGE_GAP_EM * font_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Width the notification row alone asks for at `font_size`.
    fn notification_row_width(font_size: f32) -> f32 {
        let gap = ROW_GAP_EM * font_size;

        text_width("Notifications", font_size)
            + gap
            + button_row_width(
                NotificationSource::ALL
                    .into_iter()
                    .map(|source| source.label()),
                font_size,
                gap
            )
    }

    #[test]
    fn the_window_is_wide_enough_for_every_notification_source() {
        let font_size = 16.0;

        assert!(desired_width(font_size) >= notification_row_width(font_size));
    }

    #[test]
    fn the_notification_row_is_measured_from_all_three_sources() {
        let font_size = 16.0;
        let gap = ROW_GAP_EM * font_size;

        assert_eq!(NotificationSource::ALL.len(), 3);
        assert_eq!(
            notification_row_width(font_size),
            text_width("Notifications", font_size)
                + gap
                + button_row_width(["Built in", "Hyprland", "System daemon"], font_size, gap)
        );
    }

    #[test]
    fn every_notification_source_has_room_for_its_name() {
        let font_size = 16.0;

        for source in NotificationSource::ALL {
            assert!(desired_width(font_size) >= text_width(source.label(), font_size));
        }
    }
}
