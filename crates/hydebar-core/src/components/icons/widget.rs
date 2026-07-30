//! Text widgets rendering a glyph in the icon font.

use iced::widget::Text;

use super::{
    super::{scale, text::text},
    catalog::Icons,
    theme::IconTheme
};

/// How much of its box a glyph of this range is drawn to fill.
///
/// The symbol font gathers icons drawn by different hands: a Material glyph
/// fills its box, a classic Font Awesome or Codicons glyph sits in about
/// three quarters of it. Stated per range and divided out, so glyphs from
/// different ranges standing side by side on the bar read as one size.
fn ink_share(glyph: &str) -> f32 {
    match glyph.chars().next().map_or(0, u32::from) {
        0xE200..=0xE2A9 => 0.9,
        0xE300..=0xE3E3 => 0.8,
        0xE5FA..=0xE6B7 => 0.8,
        0xE700..=0xE8EF => 0.8,
        0xEA60..=0xEC1E => 0.7,
        0xED00..=0xF2FF => 0.75,
        0xF400..=0xF533 => 0.8,
        _ => 1.0
    }
}

/// Size a glyph is stated at so its ink comes out at `size`.
fn optical(size: Option<f32>, glyph: &str) -> f32 {
    size.unwrap_or_else(scale::base) / ink_share(glyph)
}

/// Renders `icon` at the size the table carries.
pub fn icon<'a>(theme: &IconTheme, icon: Icons) -> Text<'a> {
    let glyph = theme.glyph(icon).to_owned();
    let size = optical(theme.size(), &glyph);

    bare(glyph).size(size)
}

/// Renders an arbitrary glyph, at the themed size.
pub fn icon_raw<'a>(glyph: String) -> Text<'a> {
    let size = optical(None, &glyph);

    bare(glyph).size(size)
}

/// Renders `glyph` in the icon font at `size` pixels of ink.
pub fn icon_raw_sized<'a>(glyph: String, size: Option<f32>) -> Text<'a> {
    let size = optical(size, &glyph);

    bare(glyph).size(size)
}

/// The glyph as a text widget, before any size is stated.
///
/// No font is named here on purpose: the glyph renders in the themed font and
/// falls back through the system font database when that font lacks it — the
/// same road the reference waybar theme takes, where no icon font is declared
/// anywhere and the symbols resolve to whatever nerd font the system carries.
fn bare<'a>(glyph: String) -> Text<'a> {
    text(glyph)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_box_glyph_keeps_the_stated_size() {
        assert_eq!(optical(Some(20.0), "\u{f018a}"), 20.0);
    }

    #[test]
    fn a_three_quarter_glyph_is_stated_larger_to_ink_the_same() {
        assert!(optical(Some(20.0), "\u{f011}") > 20.0);
    }

    #[test]
    fn plain_text_is_left_alone() {
        assert_eq!(optical(Some(20.0), "5"), 20.0);
    }
}
