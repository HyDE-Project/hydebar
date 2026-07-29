//! Appearance page of the settings window.

use iced::Element;

use crate::{
    components::page::{
        metrics::row_width,
        style,
        widgets::{choice_row, page, rows as row_stack, section, stepper_row}
    },
    config::{
        Appearance, AppearanceStyle, BarLayer, Config, DEFAULT_FONT_SIZE, NotificationSource,
        Position
    },
    modules::settings::{Message, Settings}
};

/// Height the bar falls back to while the configuration names none.
const FALLBACK_HEIGHT: f32 = 34.0;

/// Title of the section deciding where on the screen the bar sits.
const PLACEMENT: &str = "Placement";

/// Title of the section deciding how large and how solid the bar is drawn.
const SIZE: &str = "Size and colour";

/// Title of the section about the desktop the bar sits on.
const DESKTOP: &str = "Desktop";

/// Every section of the page, as its title and the rows it holds.
///
/// Each row is written down as its label and the controls beside it, so the
/// width the page asks for and the number of rows it reserves height for both
/// come from the same list: a row added to the page without an entry here would
/// be measured out of existence.
/// Rows the size section drops while the bar scales itself.
///
/// Height, side padding and text size are then decided from the screen, so a
/// stepper offering to change them would be offering something that is
/// overwritten the moment it is written.
const SCALED_ROWS: f32 = 3.0;

const SECTIONS: [(&str, &[(&str, &[&str])]); 3] = [
    (
        PLACEMENT,
        &[
            ("Position", &["Top", "Bottom"]),
            ("Layer", &["Bottom", "Top", "Overlay"])
        ]
    ),
    (
        SIZE,
        &[
            ("Style", &["Islands", "Solid", "Gradient"]),
            ("Height", &["\u{2212}", "000", "+"]),
            ("Side padding", &["\u{2212}", "000", "+"]),
            ("Font size", &["\u{2212}", "000", "+"]),
            ("Opacity", &["\u{2212}", "0.00", "+"])
        ]
    ),
    (DESKTOP, &[])
];

/// Label of the row the notification source is picked on.
///
/// Kept out of [`SECTIONS`] because its choices are named by the source list
/// itself rather than written down here.
const NOTIFICATIONS: &str = "Notifications";

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

    let placement = row_stack(font_size)
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
        ));

    let size = row_stack(font_size)
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
        ));

    let desktop = row_stack(font_size).push(choice_row(
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
    ));

    page(font_size)
        .push(section(PLACEMENT, placement.into(), font_size))
        .push(section(SIZE, size.into(), font_size))
        .push(section(DESKTOP, desktop.into(), font_size))
        .into()
}

/// Rows this page draws, its section headings counted in.
///
/// The notification row is the one that is not written down in [`SECTIONS`], so
/// it is added here rather than baked into a literal that could drift.
#[must_use]
pub(super) fn rows(auto_scale: bool) -> f32 {
    let settings: usize = SECTIONS.iter().map(|(_, rows)| rows.len()).sum();
    let scaled = if auto_scale { SCALED_ROWS } else { 0.0 };

    SECTIONS.len() as f32 * style::SECTION_TITLE_ROWS + settings as f32 + 1.0 - scaled
}

/// Longest row of this page, which is how wide the window has to be.
///
/// The notification row is measured from the choices themselves rather than
/// from a copy of their names: a fourth source, or a renamed one, would
/// otherwise be cut off by a window sized for the old list.
#[must_use]
pub(super) fn desired_width(font_size: f32) -> f32 {
    let notifications = row_width(
        NotificationSource::ALL
            .into_iter()
            .map(NotificationSource::label),
        font_size
    );

    SECTIONS
        .into_iter()
        .flat_map(|(_, rows)| rows.iter())
        .map(|(_, controls)| row_width(controls.iter().copied(), font_size))
        .fold(notifications, f32::max)
}

/// Height this page needs.
#[must_use]
pub(super) fn desired_height(font_size: f32, auto_scale: bool) -> f32 {
    style::page_height(rows(auto_scale), font_size)
}

#[cfg(test)]
mod tests {
    use super::{super::metrics::text_width, *};

    /// Every label this page draws in the shared label column.
    fn labels() -> Vec<&'static str> {
        SECTIONS
            .into_iter()
            .flat_map(|(_, rows)| rows.iter().map(|(label, _)| *label))
            .chain(std::iter::once(NOTIFICATIONS))
            .collect()
    }

    #[test]
    fn the_window_is_wide_enough_for_every_notification_source() {
        let font_size = 16.0;
        let notifications = row_width(
            NotificationSource::ALL
                .into_iter()
                .map(NotificationSource::label),
            font_size
        );

        assert!(desired_width(font_size) >= notifications);
    }

    #[test]
    fn the_notification_row_is_measured_from_all_three_sources() {
        assert_eq!(NotificationSource::ALL.len(), 3);
    }

    #[test]
    fn every_notification_source_has_room_for_its_name() {
        let font_size = 16.0;

        for source in NotificationSource::ALL {
            assert!(desired_width(font_size) >= text_width(source.label(), font_size));
        }
    }

    #[test]
    fn every_row_label_fits_the_shared_label_column() {
        let font_size = 16.0;

        for label in labels() {
            assert!(
                text_width(label, font_size) <= style::label_width(font_size),
                "{label} overflows the label column"
            );
        }
    }

    #[test]
    fn the_page_reserves_a_row_for_every_row_and_every_heading_it_draws() {
        assert_eq!(rows(false), labels().len() as f32 + SECTIONS.len() as f32);
        assert_eq!(rows(true), rows(false) - SCALED_ROWS);
    }

    #[test]
    fn every_section_carries_a_title() {
        for (title, _) in SECTIONS {
            assert!(!title.is_empty());
        }
    }

    #[test]
    fn the_desktop_section_holds_only_the_row_named_elsewhere() {
        // its single row is the notification source, whose choices come from
        // the source list rather than from the table, and the row count adds it
        // back on its own
        let desktop = SECTIONS
            .iter()
            .find(|(title, _)| *title == DESKTOP)
            .expect("the desktop section is drawn");

        assert!(desktop.1.is_empty());
    }

    #[test]
    fn nothing_the_bar_decides_for_itself_is_offered() {
        // following the desktop theme and scaling to the screen are not
        // choices: the bar does both, always, and a switch that pretended
        // otherwise would be a switch the bar ignores
        for (_, section_rows) in SECTIONS {
            for (label, _) in section_rows {
                assert_ne!(*label, "Follow HyDE theme");
                assert_ne!(*label, "Scale to the screen");
            }
        }
    }

    #[test]
    fn the_page_height_follows_the_shared_row_pitch() {
        let font_size = 16.0;

        assert_eq!(
            desired_height(font_size, false),
            style::page_height(rows(false), font_size)
        );
    }
}
