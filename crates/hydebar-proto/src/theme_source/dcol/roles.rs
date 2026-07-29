//! Mapping from a wallbash palette to the roles the bar paints with.
//!
//! Wallbash hands every consumer the same 44 colours and lets a per-consumer
//! template pick which of them means "background" and how transparent it is.
//! The bar is one of those consumers, so the picks are restated here verbatim
//! from the template HyDE ships for a bar
//! (`~/.local/share/wallbash/theme/waybar.dcol`). Copying the numbers rather
//! than inventing new ones is deliberate: the bar has to look like the desktop
//! it sits on, and the desktop is coloured from that template.

use super::{super::color::Rgba, palette::DcolPalette};

/// Alpha the bar surface itself is painted with.
///
/// Very nearly transparent: the islands carry the colour, the surface between
/// them only tints whatever is behind the bar.
const BAR_BACKGROUND_ALPHA: f32 = 0.01;

/// Alpha an island is painted with.
const MODULE_BACKGROUND_ALPHA: f32 = 0.8;

/// Alpha the default label colour is painted with.
const TEXT_ALPHA: f32 = 0.8;

/// Alpha an active island is painted with.
const ACTIVE_BACKGROUND_ALPHA: f32 = 0.4;

/// Alpha the label of an active island is painted with.
const ACTIVE_TEXT_ALPHA: f32 = 1.0;

/// Alpha a hovered island is painted with.
const HOVER_BACKGROUND_ALPHA: f32 = 0.4;

/// Alpha the label of a hovered island is painted with.
const HOVER_TEXT_ALPHA: f32 = 0.8;

/// The seven colours a bar is built out of.
///
/// Named after the roles rather than after the wallbash keys, because which key
/// fills which role is exactly what this module decides and a caller must not
/// have to know it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::theme_source) struct BarColors {
    /// Surface of the bar itself.
    pub bar_background:    Rgba,
    /// Surface of one island.
    pub module_background: Rgba,
    /// Default label colour.
    pub text:              Rgba,
    /// Surface of an island that is active.
    pub active_background: Rgba,
    /// Label colour of an island that is active.
    pub active_text:       Rgba,
    /// Surface of an island under the pointer.
    pub hover_background:  Rgba,
    /// Label colour of an island under the pointer.
    pub hover_text:        Rgba
}

impl BarColors {
    /// Picks the bar colours out of a wallbash palette.
    #[must_use]
    pub(super) fn from_palette(palette: &DcolPalette) -> Self {
        Self {
            bar_background:    with_alpha(palette.primary[0], BAR_BACKGROUND_ALPHA),
            module_background: with_alpha(palette.primary[0], MODULE_BACKGROUND_ALPHA),
            text:              with_alpha(palette.accents[0][7], TEXT_ALPHA),
            active_background: with_alpha(palette.primary[3], ACTIVE_BACKGROUND_ALPHA),
            active_text:       with_alpha(palette.accents[3][8], ACTIVE_TEXT_ALPHA),
            hover_background:  with_alpha(palette.accents[1][2], HOVER_BACKGROUND_ALPHA),
            hover_text:        with_alpha(palette.accents[2][7], HOVER_TEXT_ALPHA)
        }
    }
}

/// Restates a palette colour at the transparency its role calls for.
///
/// A `.dcol` holds opaque channels only; the alpha lives in the template, which
/// is why it is applied here and not while parsing.
fn with_alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba::rgba(color.r, color.g, color.b, alpha)
}

#[cfg(test)]
mod tests {
    use super::{super::fixtures::WALL_DCOL, *};

    fn colors() -> BarColors {
        BarColors::from_palette(&DcolPalette::parse(WALL_DCOL).expect("palette"))
    }

    #[test]
    fn every_role_takes_the_key_the_bar_template_names() {
        let colors = colors();

        assert_eq!(colors.bar_background, Rgba::rgba(72, 56, 40, 0.01));
        assert_eq!(colors.module_background, Rgba::rgba(72, 56, 40, 0.8));
        assert_eq!(colors.text, Rgba::rgba(240, 205, 170, 0.8));
        assert_eq!(colors.active_background, Rgba::rgba(4, 3, 2, 0.4));
        assert_eq!(colors.active_text, Rgba::rgba(73, 56, 39, 1.0));
        assert_eq!(colors.hover_background, Rgba::rgba(125, 99, 75, 0.4));
        assert_eq!(colors.hover_text, Rgba::rgba(239, 203, 169, 0.8));
    }

    #[test]
    fn the_bar_surface_and_an_island_share_one_colour_at_two_alphas() {
        let colors = colors();

        assert_eq!(
            (
                colors.bar_background.r,
                colors.bar_background.g,
                colors.bar_background.b
            ),
            (
                colors.module_background.r,
                colors.module_background.g,
                colors.module_background.b
            )
        );
        assert!(colors.bar_background.a < colors.module_background.a);
    }

    #[test]
    fn an_inverted_palette_paints_the_bar_from_the_mirrored_colour() {
        let palette = DcolPalette::parse(WALL_DCOL).expect("palette");
        let inverted = BarColors::from_palette(&palette.inverted());

        assert_eq!(
            (
                inverted.module_background.r,
                inverted.module_background.g,
                inverted.module_background.b
            ),
            (
                palette.primary[3].r,
                palette.primary[3].g,
                palette.primary[3].b
            )
        );
    }
}
