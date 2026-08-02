//! Style of the workspace indicator buttons.

use iced::{
    Background, Border, Color, Theme,
    theme::palette,
    widget::button::{self, Status}
};

use crate::config::AppearanceColor;

/// Base and strong colour pairs a workspace indicator paints with.
///
/// Derived once per style call rather than once per status arm: the palette
/// generation walks colour spaces, and the hovered arm needs the same
/// derivation as the resting one.
struct IndicatorColors {
    base:   (Color, Color),
    strong: (Color, Color)
}

/// Resolves the colour pairs for the configured monitor colour, if any.
#[expect(
    clippy::option_option,
    reason = "distinguishes unconfigured, default-coloured and custom-coloured indicators"
)]
fn indicator_colors(theme: &Theme, colors: Option<Option<AppearanceColor>>) -> IndicatorColors {
    let extended = theme.extended_palette();

    match colors {
        None => IndicatorColors {
            base:   (extended.background.weak.color, theme.palette().text),
            strong: (extended.background.strong.color, theme.palette().text)
        },
        Some(None) => IndicatorColors {
            base:   (extended.primary.base.color, extended.primary.base.text),
            strong: (extended.primary.strong.color, extended.primary.strong.text)
        },
        Some(Some(color)) => {
            let generated = palette::Primary::generate(
                color.get_base(),
                theme.palette().background,
                color.get_text().unwrap_or_else(|| theme.palette().text)
            );

            IndicatorColors {
                base:   (generated.base.color, generated.base.text),
                strong: (generated.strong.color, generated.strong.text)
            }
        }
    }
}

/// Builds the workspace button style closure, handling optional colours.
///
/// Only the focused workspace is filled with its monitor colour; the remaining
/// ones stay muted so the active one reads at a glance. A muted indicator
/// keeps the bar text colour, which is what the reference waybar theme paints
/// an idle `#workspaces button` with.
pub fn workspace_button_style(
    is_empty: bool,
    is_active: bool,
    is_urgent: bool,
    radius: f32,
    colors: Option<Option<AppearanceColor>>
) -> impl Fn(&Theme, Status) -> button::Style {
    let is_muted = is_empty || !is_active;

    move |theme: &Theme, status: Status| {
        if is_urgent && !is_active {
            let danger = theme.extended_palette().danger;
            let filled = match status {
                Status::Hovered => danger.strong,
                _ => danger.base
            };

            return button::Style {
                background: Some(Background::Color(filled.color)),
                border: Border {
                    width:  0.0,
                    color:  filled.color,
                    radius: radius.into()
                },
                text_color: filled.text,
                ..button::Style::default()
            };
        }

        let indicator = indicator_colors(theme, colors);
        let (bg_color, fg_color) = indicator.base;

        let mut base = button::Style {
            background: if is_muted {
                None
            } else {
                Some(Background::Color(bg_color))
            },
            border: Border {
                width:  if is_empty { 1.0 } else { 0.0 },
                color:  bg_color,
                radius: radius.into()
            },
            text_color: if is_muted {
                theme.palette().text
            } else {
                fg_color
            },
            ..button::Style::default()
        };

        if matches!(status, Status::Hovered) {
            let (strong_background, strong_text) = indicator.strong;

            base.background = Some(Background::Color(if is_empty {
                theme.extended_palette().background.strong.color
            } else {
                strong_background
            }));
            base.text_color = if is_empty {
                theme.palette().text
            } else {
                strong_text
            };
        }

        base
    }
}
