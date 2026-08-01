//! Border and shadow the islands adopt from the compositor's own windows.

use hydebar_proto::config::{Appearance, WindowBorder, WindowShadow};
use iced::{Border, Color, Shadow, Vector};

/// What an island wears beyond its fill: the window border and shadow of the
/// desktop it stands on, when the desktop wears them.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct IslandFinish {
    border: Option<WindowBorder>,
    shadow: Option<WindowShadow>
}

impl IslandFinish {
    /// The finish the appearance adopted from the compositor.
    #[must_use]
    pub fn of(appearance: &Appearance) -> Self {
        Self {
            border: appearance.window_border,
            shadow: appearance.window_shadow
        }
    }

    /// A finish wearing nothing, for surfaces that must stay bare.
    #[must_use]
    pub fn bare() -> Self {
        Self::default()
    }

    /// The border an island draws, rounded to `radius`.
    #[must_use]
    pub fn border(&self, radius: f32) -> Border {
        match self.border {
            Some(border) => Border {
                width:  border.width,
                color:  rgba(border.color),
                radius: radius.into()
            },
            None => Border {
                width:  0.0,
                color:  Color::TRANSPARENT,
                radius: radius.into()
            }
        }
    }

    /// The shadow an island casts.
    #[must_use]
    pub fn shadow(&self) -> Shadow {
        match self.shadow {
            Some(shadow) => Shadow {
                color:       rgba(shadow.color),
                offset:      Vector::new(0.0, 0.0),
                blur_radius: shadow.range
            },
            None => Shadow::default()
        }
    }
}

/// A compositor colour as the renderer spells it.
fn rgba([r, g, b, a]: [f32; 4]) -> Color {
    Color {
        r,
        g,
        b,
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_finish_draws_no_border_and_casts_no_shadow() {
        let finish = IslandFinish::bare();

        assert_eq!(finish.border(12.0).width, 0.0);
        assert_eq!(finish.shadow().blur_radius, 0.0);
    }

    #[test]
    fn an_adopted_finish_wears_the_compositor_colours() {
        let appearance = Appearance {
            window_border: Some(WindowBorder {
                width: 2.0,
                color: [0.5, 0.25, 0.75, 0.9]
            }),
            window_shadow: Some(WindowShadow {
                range: 4.0,
                color: [0.0, 0.0, 0.0, 0.8]
            }),
            ..Appearance::default()
        };

        let finish = IslandFinish::of(&appearance);
        let border = finish.border(8.0);

        assert_eq!(border.width, 2.0);
        assert_eq!(border.color.g, 0.25);
        assert_eq!(finish.shadow().blur_radius, 4.0);
    }
}
