//! Colour effects and the shared text-input styling.
//!
//! The overlay and darkening helpers serve the menu backdrop, which washes
//! whatever stands behind an open menu; the text-input style is the one look
//! every dialog field in the bar wears.

use iced::{Border, Color, Theme, widget::text_input};

/// Returns a [`Color`] representing the menu backdrop opacity overlay.
#[must_use]
pub const fn backdrop_color(backdrop: f32) -> Color {
    Color::from_rgba(0.0, 0.0, 0.0, backdrop)
}

/// Darkens a [`Color`] by applying the provided alpha factor.
#[must_use]
pub fn darken_color(color: Color, darkening_alpha: f32) -> Color {
    let new_r = color.r * (1.0 - darkening_alpha);
    let new_g = color.g * (1.0 - darkening_alpha);
    let new_b = color.b * (1.0 - darkening_alpha);
    let new_a = (1.0 - color.a).mul_add(darkening_alpha, color.a);

    Color::from([new_r, new_g, new_b, new_a])
}

/// Computes the [`text_input::Style`] for the given [`text_input::Status`].
#[must_use]
pub fn text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut base = text_input::Style {
        background:  theme.palette().background.into(),
        border:      Border {
            width:  2.0,
            radius: 32.0.into(),
            color:  theme.extended_palette().background.weak.color
        },
        icon:        theme.palette().text,
        placeholder: theme.palette().text,
        value:       theme.palette().text,
        selection:   theme.palette().primary
    };
    match status {
        text_input::Status::Active => base,
        text_input::Status::Focused {
            is_hovered: _
        }
        | text_input::Status::Hovered => {
            base.border.color = theme.extended_palette().background.strong.color;
            base
        }
        text_input::Status::Disabled => {
            base.background = theme.extended_palette().background.weak.color.into();
            base.border.color = Color::TRANSPARENT;
            base
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]
    #![allow(clippy::suboptimal_flops)]

    use super::*;

    #[test]
    fn text_input_style_transitions_states() {
        let theme = Theme::Dark;

        let active = text_input_style(&theme, text_input::Status::Active);
        assert_eq!(active.border.width, 2.0);
        assert_eq!(active.border.radius, 32.0.into());
        assert_eq!(
            active.border.color,
            theme.extended_palette().background.weak.color
        );

        let hovered = text_input_style(&theme, text_input::Status::Hovered);
        assert_eq!(
            hovered.border.color,
            theme.extended_palette().background.strong.color
        );

        let disabled = text_input_style(&theme, text_input::Status::Disabled);
        assert_eq!(
            disabled.background,
            theme.extended_palette().background.weak.color.into()
        );
        assert_eq!(disabled.border.color, Color::TRANSPARENT);
    }

    #[test]
    fn backdrop_color_applies_alpha_channel() {
        let color = backdrop_color(0.42);
        assert!((color.a - 0.42).abs() < f32::EPSILON);
        assert!(color.r.abs() < f32::EPSILON);
        assert!(color.g.abs() < f32::EPSILON);
        assert!(color.b.abs() < f32::EPSILON);
    }

    #[test]
    fn darken_color_scales_channels() {
        let color = Color::from_rgb(0.8, 0.6, 0.4);
        let darkened = darken_color(color, 0.5);

        assert!((darkened.r - 0.4).abs() < 0.0001);
        assert!((darkened.g - 0.3).abs() < 0.0001);
        assert!((darkened.b - 0.2).abs() < 0.0001);
        assert!((darkened.a - (color.a + (1.0 - color.a) * 0.5)).abs() < 0.0001);
    }
}
