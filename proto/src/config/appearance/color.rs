//! Palette entries of the appearance configuration and the default palette the
//! bar falls back to.

use hex_color::HexColor;
use serde::Deserialize;

use crate::theme_source::Rgba;

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

/// Reads a configured hex value as the colour the rest of the bar speaks in.
const fn opaque(color: HexColor) -> Rgba {
    Rgba::rgb(color.r, color.g, color.b)
}

impl AppearanceColor {
    /// Returns the base colour of the configured palette entry.
    #[must_use]
    pub const fn get_base(&self) -> Rgba {
        match self {
            Self::Simple(color) => opaque(*color),
            Self::Complete {
                base, ..
            } => opaque(*base)
        }
    }

    /// Returns the text colour of the entry, when it names one.
    #[must_use]
    pub const fn get_text(&self) -> Option<Rgba> {
        match self {
            Self::Simple(_) => None,
            Self::Complete {
                text, ..
            } => match text {
                Some(color) => Some(opaque(*color)),
                None => None
            }
        }
    }

    /// Returns the weak variant of the entry, when it names one.
    ///
    /// The text colour that goes with it is [`AppearanceColor::get_text`]; the
    /// two are paired by the renderer, which is the layer that knows how to
    /// keep a text colour readable over a background.
    #[must_use]
    pub const fn get_weak(&self) -> Option<Rgba> {
        match self {
            Self::Simple(_) => None,
            Self::Complete {
                weak, ..
            } => match weak {
                Some(color) => Some(opaque(*color)),
                None => None
            }
        }
    }

    /// Returns the strong variant of the entry, when it names one.
    #[must_use]
    pub const fn get_strong(&self) -> Option<Rgba> {
        match self {
            Self::Simple(_) => None,
            Self::Complete {
                strong, ..
            } => match strong {
                Some(color) => Some(opaque(*color)),
                None => None
            }
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn an_entry_naming_no_text_leaves_the_shade_to_the_renderer() {
        let color = AppearanceColor::Complete {
            base:   HexColor::rgb(1, 2, 3),
            strong: Some(HexColor::rgb(4, 5, 6)),
            weak:   Some(HexColor::rgb(7, 8, 9)),
            text:   None
        };

        assert_eq!(color.get_strong(), Some(Rgba::rgb(4, 5, 6)));
        assert_eq!(color.get_weak(), Some(Rgba::rgb(7, 8, 9)));
        assert_eq!(color.get_text(), None);
    }

    #[test]
    fn a_simple_entry_names_nothing_but_its_base() {
        let color = AppearanceColor::Simple(HexColor::rgb(1, 2, 3));

        assert_eq!(color.get_base(), Rgba::rgb(1, 2, 3));
        assert_eq!(color.get_strong(), None);
        assert_eq!(color.get_weak(), None);
        assert_eq!(color.get_text(), None);
    }
}
