use iced::{Border, Theme, widget::container::Style};

use super::theme::backdrop_color;
use crate::config::Position;

fn joined_radius(radius: f32, bar_position: Option<Position>) -> iced::border::Radius {
    let r = radius.max(0.0);
    match bar_position {
        Some(Position::Top) => iced::border::Radius {
            top_left:     0.0,
            top_right:    0.0,
            bottom_right: r,
            bottom_left:  r
        },
        Some(Position::Bottom) => iced::border::Radius {
            top_left:     r,
            top_right:    r,
            bottom_right: 0.0,
            bottom_left:  0.0
        },
        Some(Position::Left) => iced::border::Radius {
            top_left:     0.0,
            top_right:    r,
            bottom_right: r,
            bottom_left:  0.0
        },
        Some(Position::Right) => iced::border::Radius {
            top_left:     r,
            top_right:    0.0,
            bottom_right: 0.0,
            bottom_left:  r
        },
        None => radius.into()
    }
}

/// Builds the menu container style closure used for popup content.
///
/// The radius is the themed pill radius rather than a constant of its own: a
/// menu rounded differently from the pills it opens from, and from the windows
/// the compositor rounds beside it, reads as a foreign surface. When joined
/// to a bar edge, the corners on that edge are square so the drawer reads as
/// attached to the bar.
pub fn menu_container_style(
    opacity: f32,
    radius: f32,
    finish: super::IslandFinish,
    bar_position: Option<Position>
) -> impl Fn(&Theme) -> Style {
    move |theme: &Theme| {
        let adopted = finish.border(radius);
        let joined = joined_radius(radius, bar_position);

        Style {
            background: Some(theme.palette().background.scale_alpha(opacity).into()),
            border: if adopted.width > 0.0 {
                Border {
                    radius: joined,
                    ..adopted
                }
            } else {
                Border {
                    color:  theme
                        .extended_palette()
                        .secondary
                        .base
                        .color
                        .scale_alpha(opacity),
                    width:  1.0,
                    radius: joined
                }
            },
            shadow: finish.shadow(),
            ..Style::default()
        }
    }
}

/// Builds the tooltip container style closure used for the hover popups.
///
/// A tooltip is drawn exactly like a menu box: the same colors, the same
/// themed corner radius.
pub fn tooltip_container_style(opacity: f32, radius: f32) -> impl Fn(&Theme) -> Style {
    menu_container_style(opacity, radius, super::IslandFinish::bare(), None)
}

/// Builds the menu backdrop style closure that applies the configured opacity.
pub fn menu_backdrop_style(backdrop: f32) -> impl Fn(&Theme) -> Style {
    move |_| Style {
        background: Some(backdrop_color(backdrop).into()),
        ..Style::default()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::{Background, Color};

    use super::*;

    fn color(background: Option<Background>) -> Color {
        match background.expect("background should be set") {
            Background::Color(color) => color,
            other @ Background::Gradient(_) => panic!("unexpected background: {other:?}")
        }
    }

    #[test]
    fn menu_container_style_scales_opacity_and_takes_the_themed_radius() {
        let theme = Theme::Dark;
        let style_fn = menu_container_style(0.3, 12.0, super::super::IslandFinish::bare(), None);
        let style = style_fn(&theme);

        let background = color(style.background);
        assert_eq!(background.a, 0.3 * theme.palette().background.a);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius, 12.0.into());
    }

    #[test]
    fn menu_backdrop_style_uses_backdrop_color() {
        let theme = Theme::Dark;
        let style_fn = menu_backdrop_style(0.6);
        let style = style_fn(&theme);

        let background = color(style.background);
        assert!((background.a - 0.6).abs() < f32::EPSILON);
    }
}
