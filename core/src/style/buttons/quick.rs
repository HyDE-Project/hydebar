//! Style of the quick settings buttons.

use iced::{
    Border, Color, Theme,
    widget::button::{self, Status}
};

/// Builds the quick settings button style closure with active feedback.
pub fn quick_settings_button_style(
    is_active: bool,
    opacity: f32
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let mut base = button::Style {
            background: Some(
                if is_active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.color
                }
                .scale_alpha(opacity)
                .into()
            ),
            border: Border {
                width:  0.0,
                radius: 32.0.into(),
                color:  Color::TRANSPARENT
            },
            text_color: if is_active {
                theme.extended_palette().primary.base.text
            } else {
                theme.palette().text
            },
            ..button::Style::default()
        };
        match status {
            Status::Hovered => {
                let peach = theme.extended_palette().primary.weak.color;
                base.background = Some(
                    if is_active {
                        peach
                    } else {
                        theme.extended_palette().background.strong.color
                    }
                    .scale_alpha(opacity)
                    .into()
                );
                base
            }
            _ => base
        }
    }
}

/// Builds the submenu button style closure used inside quick settings menus.
pub fn quick_settings_submenu_button_style(
    is_active: bool,
    opacity: f32
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width:  0.0,
                radius: 16.0.into(),
                color:  Color::TRANSPARENT
            },
            text_color: if is_active {
                theme.extended_palette().primary.base.text
            } else {
                theme.palette().text
            },
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
                base.text_color = theme.palette().text;
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

    fn background_color(style: &button::Style) -> Option<Color> {
        match style.background {
            Some(Background::Color(color)) => Some(color),
            _ => None
        }
    }

    #[test]
    fn an_idle_quick_button_wears_the_weak_background_and_bar_text() {
        let theme = Theme::Dark;
        let resting = quick_settings_button_style(false, 1.0)(&theme, Status::Active);

        assert_eq!(
            background_color(&resting),
            Some(theme.extended_palette().background.weak.color)
        );
        assert_eq!(resting.text_color, theme.palette().text);
        assert_eq!(resting.border.radius, 32.0.into());
        assert_eq!(resting.border.width, 0.0);
    }

    #[test]
    fn an_active_quick_button_wears_the_primary_fill_and_its_text() {
        let theme = Theme::Dark;
        let resting = quick_settings_button_style(true, 1.0)(&theme, Status::Active);

        assert_eq!(background_color(&resting), Some(theme.palette().primary));
        assert_eq!(
            resting.text_color,
            theme.extended_palette().primary.base.text
        );
    }

    #[test]
    fn hovering_an_idle_quick_button_deepens_the_background() {
        let theme = Theme::Dark;
        let hovered = quick_settings_button_style(false, 1.0)(&theme, Status::Hovered);

        assert_eq!(
            background_color(&hovered),
            Some(theme.extended_palette().background.strong.color)
        );
    }

    #[test]
    fn hovering_an_active_quick_button_lightens_it_to_the_weak_primary() {
        let theme = Theme::Dark;
        let hovered = quick_settings_button_style(true, 1.0)(&theme, Status::Hovered);

        assert_eq!(
            background_color(&hovered),
            Some(theme.extended_palette().primary.weak.color)
        );
    }

    #[test]
    fn pressed_and_disabled_quick_buttons_keep_the_resting_fill() {
        let theme = Theme::Dark;
        let styled = quick_settings_button_style(false, 1.0);
        let resting = background_color(&styled(&theme, Status::Active));

        for status in [Status::Pressed, Status::Disabled] {
            assert_eq!(background_color(&styled(&theme, status)), resting);
        }
    }

    #[test]
    fn the_quick_button_opacity_fades_every_fill() {
        let theme = Theme::Dark;
        let faded = background_color(&quick_settings_button_style(true, 0.3)(
            &theme,
            Status::Active
        ))
        .map(|color| color.a);

        assert_eq!(faded, Some(theme.palette().primary.scale_alpha(0.3).a));
    }

    #[test]
    fn a_resting_submenu_button_paints_nothing() {
        let theme = Theme::Dark;
        let resting = quick_settings_submenu_button_style(false, 1.0)(&theme, Status::Active);

        assert!(resting.background.is_none());
        assert_eq!(resting.border.radius, 16.0.into());
        assert_eq!(resting.border.color, Color::TRANSPARENT);
        assert_eq!(resting.text_color, theme.palette().text);
    }

    #[test]
    fn an_active_submenu_button_takes_the_primary_text_colour() {
        let theme = Theme::Dark;
        let resting = quick_settings_submenu_button_style(true, 1.0)(&theme, Status::Active);

        assert_eq!(
            resting.text_color,
            theme.extended_palette().primary.base.text
        );
    }

    #[test]
    fn hovering_a_submenu_button_fades_in_the_weak_background_and_bar_text() {
        let theme = Theme::Dark;
        let hovered = quick_settings_submenu_button_style(true, 0.5)(&theme, Status::Hovered);

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
        assert_eq!(hovered.text_color, theme.palette().text);
    }

    #[test]
    fn pressed_and_disabled_submenu_buttons_stay_unpainted() {
        let styled = quick_settings_submenu_button_style(true, 1.0);

        for status in [Status::Pressed, Status::Disabled] {
            assert!(styled(&Theme::Dark, status).background.is_none());
        }
    }
}
