//! Palette entries of the appearance configuration and the default palette the
//! bar falls back to.

use hex_color::HexColor;
use iced::{Color, theme::palette};
use serde::Deserialize;

/// Color palette configuration used to render UI elements.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum AppearanceColor {
    /// Simple color variant with a single hex value.
    Simple(HexColor),
    /// Complete palette variant with additional semantic colors.
    Complete {
        base:   HexColor,
        strong: Option<HexColor>,
        weak:   Option<HexColor>,
        text:   Option<HexColor>
    }
}

impl AppearanceColor {
    /// Returns the base [`Color`] representation for the configured palette.
    #[must_use]
    pub const fn get_base(&self) -> Color {
        match self {
            Self::Simple(color) => Color::from_rgb8(color.r, color.g, color.b),
            Self::Complete {
                base, ..
            } => Color::from_rgb8(base.r, base.g, base.b)
        }
    }

    /// Returns the text [`Color`] if configured.
    #[must_use]
    pub fn get_text(&self) -> Option<Color> {
        match self {
            Self::Simple(_) => None,
            Self::Complete {
                text, ..
            } => text.map(|color| Color::from_rgb8(color.r, color.g, color.b))
        }
    }

    /// Builds the weak [`palette::Pair`] variant if available.
    #[must_use]
    pub fn get_weak_pair(&self, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            Self::Simple(_) => None,
            Self::Complete {
                weak,
                text,
                ..
            } => weak.map(|color| {
                palette::Pair::new(
                    Color::from_rgb8(color.r, color.g, color.b),
                    text.map_or(text_fallback, |color| {
                        Color::from_rgb8(color.r, color.g, color.b)
                    })
                )
            })
        }
    }

    /// Builds the strong [`palette::Pair`] variant if available.
    #[must_use]
    pub fn get_strong_pair(&self, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            Self::Simple(_) => None,
            Self::Complete {
                strong,
                text,
                ..
            } => strong.map(|color| {
                palette::Pair::new(
                    Color::from_rgb8(color.r, color.g, color.b),
                    text.map_or(text_fallback, |color| {
                        Color::from_rgb8(color.r, color.g, color.b)
                    })
                )
            })
        }
    }
}

static PRIMARY: HexColor = HexColor::rgb(250, 179, 135);

pub(super) const fn default_background_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base:   HexColor::rgb(30, 30, 46),
        strong: Some(HexColor::rgb(69, 71, 90)),
        weak:   Some(HexColor::rgb(49, 50, 68)),
        text:   None
    }
}

pub(super) fn default_primary_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base:   PRIMARY,
        strong: None,
        weak:   None,
        text:   Some(HexColor::rgb(30, 30, 46))
    }
}

pub(super) const fn default_secondary_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base:   HexColor::rgb(17, 17, 27),
        strong: Some(HexColor::rgb(24, 24, 37)),
        weak:   None,
        text:   None
    }
}

pub(super) const fn default_success_color() -> AppearanceColor {
    AppearanceColor::Simple(HexColor::rgb(166, 227, 161))
}

pub(super) const fn default_danger_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base:   HexColor::rgb(243, 139, 168),
        weak:   Some(HexColor::rgb(249, 226, 175)),
        strong: None,
        text:   None
    }
}

pub(super) const fn default_warning_color() -> AppearanceColor {
    AppearanceColor::Simple(HexColor::rgb(250, 179, 135))
}

pub(super) const fn default_text_color() -> AppearanceColor {
    AppearanceColor::Simple(HexColor::rgb(205, 214, 244))
}

pub(super) fn default_workspace_colors() -> Vec<AppearanceColor> {
    vec![
        AppearanceColor::Simple(PRIMARY),
        AppearanceColor::Simple(HexColor::rgb(180, 190, 254)),
        AppearanceColor::Simple(HexColor::rgb(203, 166, 247)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_color_pairs_use_text_fallback() {
        let fallback = Color::from_rgb8(255, 255, 255);
        let color = AppearanceColor::Complete {
            base:   HexColor::rgb(1, 2, 3),
            strong: Some(HexColor::rgb(4, 5, 6)),
            weak:   Some(HexColor::rgb(7, 8, 9)),
            text:   None
        };

        let strong = color.get_strong_pair(fallback).expect("strong pair");
        assert_eq!(strong.text, fallback);

        let weak = color.get_weak_pair(fallback).expect("weak pair");
        assert_eq!(weak.text, fallback);
    }
}
