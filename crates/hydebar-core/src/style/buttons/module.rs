//! Style of the buttons hosting bar modules.

use iced::{
    Border, Color, Theme,
    widget::button::{self, Status}
};

use crate::config::AppearanceStyle;

/// Builds the module button style closure based on the appearance
/// configuration.
///
/// `radius` is the corner radius of the pill, in pixels; callers pass
/// [`Appearance::pill_radius`](crate::config::Appearance::pill_radius) so the
/// configured or HyDE provided value reaches the border.
pub fn module_button_style(
    style: AppearanceStyle,
    opacity: f32,
    radius: f32,
    transparent: bool,
    focused: bool
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: match style {
                AppearanceStyle::Solid | AppearanceStyle::Gradient => None,
                AppearanceStyle::Islands => {
                    if transparent {
                        None
                    } else {
                        Some(theme.palette().background.scale_alpha(opacity).into())
                    }
                }
            },
            border: if focused {
                Border {
                    width:  2.0,
                    radius: radius.into(),
                    color:  theme.palette().primary
                }
            } else {
                Border {
                    width:  0.0,
                    radius: radius.into(),
                    color:  Color::TRANSPARENT
                }
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Appearance, DEFAULT_RADIUS};

    #[test]
    fn the_configured_radius_reaches_the_border() {
        let theme = Theme::Dark;

        let styled = module_button_style(AppearanceStyle::Islands, 1.0, 9.0, false, false);
        assert_eq!(styled(&theme, Status::Active).border.radius, 9.0.into());

        let focused = module_button_style(AppearanceStyle::Islands, 1.0, 9.0, false, true);
        assert_eq!(focused(&theme, Status::Active).border.radius, 9.0.into());
    }

    #[test]
    fn an_unset_radius_falls_back_to_the_previous_constant() {
        let appearance = Appearance::default();
        assert_eq!(appearance.pill_radius(), DEFAULT_RADIUS);

        let styled = module_button_style(
            AppearanceStyle::Islands,
            1.0,
            appearance.pill_radius(),
            false,
            false
        );
        assert_eq!(
            styled(&Theme::Dark, Status::Active).border.radius,
            4.0.into()
        );
    }
}
