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
/// configured or `HyDE` provided value reaches the border.
///
/// `hover` is how far the hover fade of this module has travelled, zero at
/// rest and one fully lit. The background is blended from it rather than from
/// the widget's own hover status: the fade spring is what carries the pointer
/// entering and leaving, so the highlight breathes in and out instead of
/// flipping.
pub fn module_button_style(
    style: AppearanceStyle,
    opacity: f32,
    radius: f32,
    transparent: bool,
    focused: bool,
    hover: f32,
    finish: crate::style::IslandFinish
) -> impl Fn(&Theme, Status) -> button::Style {
    let island = matches!(style, AppearanceStyle::Islands) && !transparent;

    move |theme, _status| {
        let rest = match style {
            AppearanceStyle::Solid | AppearanceStyle::Gradient => None,
            AppearanceStyle::Islands => {
                if transparent {
                    None
                } else {
                    Some(theme.palette().background.scale_alpha(opacity))
                }
            }
        };
        let lit = theme
            .extended_palette()
            .background
            .weak
            .color
            .scale_alpha(opacity);

        button::Style {
            background: blend_background(rest, lit, hover.clamp(0.0, 1.0)).map(Into::into),
            border: if focused {
                Border {
                    width:  2.0,
                    radius: radius.into(),
                    color:  theme.palette().primary
                }
            } else if island {
                finish.border(radius)
            } else {
                Border {
                    width:  0.0,
                    radius: radius.into(),
                    color:  Color::TRANSPARENT
                }
            },
            shadow: if island {
                finish.shadow()
            } else {
                iced::Shadow::default()
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        }
    }
}

/// Mixes the resting background into the lit one by `t`.
///
/// A resting side that paints nothing fades the lit colour in through its
/// alpha, and stays truly unpainted at zero so an idle pill costs no quad.
fn blend_background(rest: Option<Color>, lit: Color, t: f32) -> Option<Color> {
    match rest {
        None if t <= 0.0 => None,
        None => Some(lit.scale_alpha(t)),
        Some(rest) => Some(Color {
            r: (lit.r - rest.r).mul_add(t, rest.r),
            g: (lit.g - rest.g).mul_add(t, rest.g),
            b: (lit.b - rest.b).mul_add(t, rest.b),
            a: (lit.a - rest.a).mul_add(t, rest.a)
        })
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;
    use crate::config::{Appearance, DEFAULT_RADIUS};

    #[test]
    fn the_configured_radius_reaches_the_border() {
        let theme = Theme::Dark;

        let styled = module_button_style(
            AppearanceStyle::Islands,
            1.0,
            9.0,
            false,
            false,
            0.0,
            crate::style::IslandFinish::bare()
        );
        assert_eq!(styled(&theme, Status::Active).border.radius, 9.0.into());

        let focused = module_button_style(
            AppearanceStyle::Islands,
            1.0,
            9.0,
            false,
            true,
            0.0,
            crate::style::IslandFinish::bare()
        );
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
            false,
            0.0,
            crate::style::IslandFinish::bare()
        );
        assert_eq!(
            styled(&Theme::Dark, Status::Active).border.radius,
            4.0.into()
        );
    }

    #[test]
    fn the_hover_fade_carries_the_background_between_rest_and_lit() {
        let theme = Theme::Dark;
        let at = |hover: f32| {
            let styled = module_button_style(
                AppearanceStyle::Islands,
                1.0,
                4.0,
                false,
                false,
                hover,
                crate::style::IslandFinish::bare()
            );
            match styled(&theme, Status::Active).background {
                Some(iced::Background::Color(color)) => color,
                other => panic!("expected a colour background, got {other:?}")
            }
        };

        let rest = at(0.0);
        let lit = at(1.0);
        let midway = at(0.5);

        assert_eq!(rest, theme.palette().background);
        assert_eq!(lit, theme.extended_palette().background.weak.color);
        assert!(midway.r > rest.r.min(lit.r) - f32::EPSILON);
        assert_ne!(midway, rest);
        assert_ne!(midway, lit);
    }

    #[test]
    fn a_transparent_pill_at_rest_paints_nothing() {
        let theme = Theme::Dark;

        let resting = module_button_style(
            AppearanceStyle::Islands,
            1.0,
            4.0,
            true,
            false,
            0.0,
            crate::style::IslandFinish::bare()
        );
        assert!(resting(&theme, Status::Active).background.is_none());

        let lit = module_button_style(
            AppearanceStyle::Islands,
            1.0,
            4.0,
            true,
            false,
            1.0,
            crate::style::IslandFinish::bare()
        );
        assert!(lit(&theme, Status::Active).background.is_some());
    }
}
