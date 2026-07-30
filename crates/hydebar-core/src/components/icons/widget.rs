//! Text widgets rendering a glyph in the icon font.

use iced::{
    Pixels,
    widget::{Text, text::LineHeight}
};

use super::{
    super::{scale, text::text},
    catalog::Icons,
    optical,
    theme::IconTheme
};

/// Line box of an icon, in em of the icon size.
///
/// Stated absolutely and identically for every glyph on purpose: the optical
/// correction states different font sizes for different glyphs, and left to
/// the renderer each would get a line box of its own height — icons standing
/// in one row would push their rows apart and drift off the common centre.
const LINE_EM: f32 = 1.2;

/// Renders `icon` at the size the table carries.
pub fn icon<'a>(theme: &IconTheme, icon: Icons) -> Text<'a> {
    let glyph = theme.glyph(icon).to_owned();
    let size = theme.size().unwrap_or_else(scale::base);

    build(glyph, size)
}

/// Renders an arbitrary glyph, at the themed size.
pub fn icon_raw<'a>(glyph: String) -> Text<'a> {
    build(glyph, scale::base())
}

/// Renders `glyph` at `size` pixels of apparent size.
pub fn icon_raw_sized<'a>(glyph: String, size: Option<f32>) -> Text<'a> {
    build(glyph, size.unwrap_or_else(scale::base))
}

/// The glyph as a text widget, optically corrected to `size`.
///
/// The stated font size comes from the glyph's own ink share — see
/// [`optical::stated_size`] — so glyphs drawn to different shares of their
/// box come out at one apparent size.
///
/// No font is named here on purpose: the glyph renders in the themed font and
/// falls back through the system font database when that font lacks it — the
/// same road the reference waybar theme takes, where no icon font is declared
/// anywhere and the symbols resolve to whatever nerd font the system carries.
fn build<'a>(glyph: String, size: f32) -> Text<'a> {
    let stated = optical::stated_size(&glyph, size);

    text(glyph)
        .size(stated)
        .line_height(LineHeight::Absolute(Pixels(size * LINE_EM)))
}
