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
///
/// No allocation: the glyph table hands out immortal references, and the
/// widget borrows one — this runs for every icon of every frame.
pub fn icon<'a>(theme: &IconTheme, icon: Icons) -> Text<'a> {
    let glyph = theme.glyph(icon);
    let size = theme.size().unwrap_or_else(scale::base);

    build(std::borrow::Cow::Borrowed(glyph), size)
}

/// Renders an arbitrary glyph, at the themed size.
#[must_use]
pub fn icon_raw<'a>(glyph: String) -> Text<'a> {
    build(std::borrow::Cow::Owned(glyph), scale::base())
}

/// Renders `glyph` at `size` pixels of apparent size.
pub fn icon_raw_sized<'a>(glyph: String, size: Option<f32>) -> Text<'a> {
    build(
        std::borrow::Cow::Owned(glyph),
        size.unwrap_or_else(scale::base)
    )
}

/// The glyph as a text widget, optically corrected to `size`.
///
/// The face and the stated font size both come from [`optical::resolved`]:
/// fontconfig names the font, the same way the reference waybar resolves its
/// icons, and the size correction is measured from that very face — so glyphs
/// drawn to different shares of their box come out at one apparent size.
///
/// The widget owns a box of the same width whatever the glyph: every font
/// pads its glyphs with side bearings of its own taste, and left to them two
/// icons standing side by side would show a different gap in every pair. The
/// ink is centred in the box, so the gap between icons is exactly the gap
/// their wrappers state.
fn build<'a>(glyph: std::borrow::Cow<'static, str>, size: f32) -> Text<'a> {
    let face = optical::resolved(&glyph);
    let styled = text(glyph)
        .size(size * face.factor)
        .line_height(LineHeight::Absolute(Pixels(size * LINE_EM)));

    match face.font {
        Some(font) => styled
            .font(font)
            .width(size * LINE_EM)
            .align_x(iced::alignment::Horizontal::Center),
        None => styled
    }
}
