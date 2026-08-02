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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    fn background_color(style: &button::Style) -> Option<Color> {
        match style.background {
            Some(Background::Color(color)) => Some(color),
            _ => None
        }
    }

    fn custom() -> AppearanceColor {
        AppearanceColor::Complete {
            base:   hex_color::HexColor::rgb(0x10, 0x20, 0x30),
            strong: None,
            weak:   None,
            text:   Some(hex_color::HexColor::rgb(0xf0, 0xf0, 0xf0))
        }
    }

    #[test]
    fn an_unconfigured_focused_workspace_fills_with_the_weak_background() {
        let theme = Theme::Dark;
        let resting = workspace_button_style(false, true, false, 8.0, None)(&theme, Status::Active);

        assert_eq!(
            background_color(&resting),
            Some(theme.extended_palette().background.weak.color)
        );
        assert_eq!(resting.border.radius, 8.0.into());
        assert_eq!(resting.border.width, 0.0);
    }

    #[test]
    fn an_idle_workspace_is_muted_and_keeps_the_bar_text() {
        let theme = Theme::Dark;
        let resting = workspace_button_style(false, false, false, 8.0, None)(&theme, Status::Active);

        assert!(resting.background.is_none());
        assert_eq!(resting.text_color, theme.palette().text);
    }

    #[test]
    fn an_empty_workspace_is_outlined_rather_than_filled() {
        let theme = Theme::Dark;
        let resting = workspace_button_style(true, true, false, 8.0, None)(&theme, Status::Active);

        assert!(resting.background.is_none());
        assert_eq!(resting.border.width, 1.0);
    }

    #[test]
    fn hovering_an_empty_workspace_uses_the_strong_background_and_bar_text() {
        let theme = Theme::Dark;
        let hovered = workspace_button_style(true, true, false, 8.0, None)(&theme, Status::Hovered);

        assert_eq!(
            background_color(&hovered),
            Some(theme.extended_palette().background.strong.color)
        );
        assert_eq!(hovered.text_color, theme.palette().text);
    }

    #[test]
    fn hovering_a_filled_workspace_deepens_it_to_the_strong_pair() {
        let theme = Theme::Dark;
        let hovered =
            workspace_button_style(false, true, false, 8.0, Some(None))(&theme, Status::Hovered);

        assert_eq!(
            background_color(&hovered),
            Some(theme.extended_palette().primary.strong.color)
        );
        assert_eq!(
            hovered.text_color,
            theme.extended_palette().primary.strong.text
        );
    }

    #[test]
    fn a_default_coloured_workspace_takes_the_primary_pair() {
        let theme = Theme::Dark;
        let resting =
            workspace_button_style(false, true, false, 8.0, Some(None))(&theme, Status::Active);

        assert_eq!(
            background_color(&resting),
            Some(theme.extended_palette().primary.base.color)
        );
        assert_eq!(
            resting.text_color,
            theme.extended_palette().primary.base.text
        );
    }

    #[test]
    fn a_custom_coloured_workspace_generates_its_own_pair() {
        let theme = Theme::Dark;
        let resting = workspace_button_style(false, true, false, 8.0, Some(Some(custom())))(
            &theme,
            Status::Active
        );

        assert_ne!(
            background_color(&resting),
            Some(theme.extended_palette().primary.base.color)
        );
        assert!(resting.background.is_some());

        let hovered = workspace_button_style(false, true, false, 8.0, Some(Some(custom())))(
            &theme,
            Status::Hovered
        );
        assert_ne!(background_color(&hovered), background_color(&resting));
    }

    #[test]
    fn an_urgent_unfocused_workspace_is_painted_danger() {
        let theme = Theme::Dark;
        let resting = workspace_button_style(false, false, true, 8.0, None)(&theme, Status::Active);
        let danger = theme.extended_palette().danger;

        assert_eq!(background_color(&resting), Some(danger.base.color));
        assert_eq!(resting.text_color, danger.base.text);
        assert_eq!(resting.border.radius, 8.0.into());
        assert_eq!(resting.border.width, 0.0);
    }

    #[test]
    fn hovering_an_urgent_workspace_deepens_the_danger_fill() {
        let theme = Theme::Dark;
        let hovered = workspace_button_style(false, false, true, 8.0, None)(&theme, Status::Hovered);
        let danger = theme.extended_palette().danger;

        assert_eq!(background_color(&hovered), Some(danger.strong.color));
        assert_eq!(hovered.text_color, danger.strong.text);
    }

    #[test]
    fn urgency_yields_to_focus() {
        let theme = Theme::Dark;
        let focused =
            workspace_button_style(false, true, true, 8.0, Some(None))(&theme, Status::Active);

        assert_eq!(
            background_color(&focused),
            Some(theme.extended_palette().primary.base.color)
        );
    }

    #[test]
    fn pressed_and_disabled_workspaces_keep_the_resting_fill() {
        let theme = Theme::Dark;
        let styled = workspace_button_style(false, true, false, 8.0, Some(None));
        let resting = background_color(&styled(&theme, Status::Active));

        for status in [Status::Pressed, Status::Disabled] {
            assert_eq!(background_color(&styled(&theme, status)), resting);
        }
    }
}
