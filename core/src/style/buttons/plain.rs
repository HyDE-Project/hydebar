//! Ghost, outline, confirm and settings button styles.

use iced::{
    Border, Color, Theme,
    widget::button::{self, Status}
};

/// Builds a ghost button style closure that fades in on hover.
pub fn ghost_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width:  0.0,
                radius: 4.0.into(),
                color:  Color::TRANSPARENT
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into()
                );
                base
            }
            _ => base
        }
    }
}

/// Builds the style of a context menu entry.
///
/// It is the ghost style of the other menu rows, rounded with the pill radius
/// the bar is themed with so an entry echoes the module it was opened from.
pub fn menu_entry_button_style(
    opacity: f32,
    radius: f32
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let base = ghost_button_style(opacity)(theme, status);

        button::Style {
            border: Border {
                radius: radius.into(),
                ..base.border
            },
            ..base
        }
    }
}

/// Builds an outline button style closure that highlights borders on hover.
pub fn outline_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width:  2.0,
                radius: 32.0.into(),
                color:  theme.extended_palette().background.weak.color
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into()
                );
                base
            }
            _ => base
        }
    }
}

/// Builds the confirm button style closure with filled background.
pub fn confirm_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity)
                    .into()
            ),
            border: Border {
                width:  2.0,
                radius: 32.0.into(),
                color:  Color::TRANSPARENT
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .strong
                        .color
                        .scale_alpha(opacity)
                        .into()
                );
                base
            }
            _ => base
        }
    }
}

/// Builds the rounded settings button style closure.
pub fn settings_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity)
                    .into()
            ),
            border: Border {
                width:  0.0,
                radius: 32.0.into(),
                color:  Color::TRANSPARENT
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .strong
                        .color
                        .scale_alpha(opacity)
                        .into()
                );
                base
            }
            _ => base
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::Background;

    use super::*;

    fn dark() -> Theme {
        Theme::Dark
    }

    fn background_color(style: &button::Style) -> Option<Color> {
        match style.background {
            Some(Background::Color(color)) => Some(color),
            _ => None
        }
    }

    #[test]
    fn a_resting_ghost_button_paints_nothing() {
        let styled = ghost_button_style(1.0);
        let resting = styled(&dark(), Status::Active);

        assert!(resting.background.is_none());
        assert_eq!(resting.border.width, 0.0);
        assert_eq!(resting.border.radius, 4.0.into());
        assert_eq!(resting.border.color, Color::TRANSPARENT);
        assert_eq!(resting.text_color, dark().palette().text);
    }

    #[test]
    fn a_hovered_ghost_button_fades_in_the_weak_background() {
        let theme = dark();
        let hovered = ghost_button_style(0.5)(&theme, Status::Hovered);

        assert_eq!(
            background_color(&hovered),
            Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(0.5)
            )
        );
    }

    #[test]
    fn pressed_and_disabled_ghost_buttons_rest_like_an_idle_one() {
        let styled = ghost_button_style(1.0);

        for status in [Status::Pressed, Status::Disabled] {
            assert!(styled(&dark(), status).background.is_none());
        }
    }

    #[test]
    fn a_menu_entry_keeps_the_ghost_paint_and_takes_the_pill_radius() {
        let theme = dark();
        let entry = menu_entry_button_style(1.0, 12.0);
        let ghost = ghost_button_style(1.0);

        let resting = entry(&theme, Status::Active);
        assert_eq!(resting.border.radius, 12.0.into());
        assert!(resting.background.is_none());

        let hovered = entry(&theme, Status::Hovered);
        assert_eq!(hovered.border.radius, 12.0.into());
        assert_eq!(
            background_color(&hovered),
            background_color(&ghost(&theme, Status::Hovered))
        );
    }

    #[test]
    fn a_resting_outline_button_shows_only_its_border() {
        let theme = dark();
        let resting = outline_button_style(1.0)(&theme, Status::Active);

        assert!(resting.background.is_none());
        assert_eq!(resting.border.width, 2.0);
        assert_eq!(resting.border.radius, 32.0.into());
        assert_eq!(
            resting.border.color,
            theme.extended_palette().background.weak.color
        );
    }

    #[test]
    fn a_hovered_outline_button_fills_with_the_weak_background() {
        let theme = dark();
        let hovered = outline_button_style(0.25)(&theme, Status::Hovered);

        assert_eq!(
            background_color(&hovered),
            Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(0.25)
            )
        );
        assert_eq!(hovered.border.width, 2.0);
    }

    #[test]
    fn pressed_and_disabled_outline_buttons_rest_like_an_idle_one() {
        let styled = outline_button_style(1.0);

        for status in [Status::Pressed, Status::Disabled] {
            assert!(styled(&dark(), status).background.is_none());
        }
    }

    #[test]
    fn a_confirm_button_deepens_from_weak_to_strong_on_hover() {
        let theme = dark();
        let styled = confirm_button_style(1.0);

        assert_eq!(
            background_color(&styled(&theme, Status::Active)),
            Some(theme.extended_palette().background.weak.color)
        );
        assert_eq!(
            background_color(&styled(&theme, Status::Hovered)),
            Some(theme.extended_palette().background.strong.color)
        );
    }

    #[test]
    fn a_confirm_button_stays_transparently_bordered() {
        let resting = confirm_button_style(1.0)(&dark(), Status::Pressed);

        assert_eq!(resting.border.width, 2.0);
        assert_eq!(resting.border.color, Color::TRANSPARENT);
        assert_eq!(resting.border.radius, 32.0.into());
    }

    #[test]
    fn a_settings_button_deepens_from_weak_to_strong_on_hover() {
        let theme = dark();
        let styled = settings_button_style(1.0);

        assert_eq!(
            background_color(&styled(&theme, Status::Active)),
            Some(theme.extended_palette().background.weak.color)
        );
        assert_eq!(
            background_color(&styled(&theme, Status::Hovered)),
            Some(theme.extended_palette().background.strong.color)
        );
    }

    #[test]
    fn a_settings_button_is_borderless_and_fully_rounded() {
        let resting = settings_button_style(1.0)(&dark(), Status::Disabled);

        assert_eq!(resting.border.width, 0.0);
        assert_eq!(resting.border.radius, 32.0.into());
        assert_eq!(resting.text_color, dark().palette().text);
    }

    #[test]
    fn the_opacity_scales_every_filled_style() {
        let theme = dark();
        let opaque = background_color(&confirm_button_style(1.0)(&theme, Status::Active))
            .map(|color| color.a);
        let faded =
            background_color(&confirm_button_style(0.4)(&theme, Status::Active)).map(|color| color.a);

        assert!(faded < opaque);
    }
}
