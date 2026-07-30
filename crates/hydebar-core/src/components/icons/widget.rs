//! Text widgets rendering a glyph in the icon font.

use iced::{Font, widget::Text};

use super::{super::text::text, catalog::Icons, theme::IconTheme};

/// Font every glyph of the catalogue is looked up in.
const ICON_FONT: &str = "Symbols Nerd Font";

/// Renders `icon` at the size the table carries.
pub fn icon<'a>(theme: &IconTheme, icon: Icons) -> Text<'a> {
    sized(icon_raw(theme.glyph(icon).to_owned()), theme.size())
}

/// Renders an arbitrary glyph in the icon font, at the themed size.
///
/// Built on the themed text helper on purpose: a glyph left at the renderer
/// default would stay small while the rest of the bar follows the screen.
pub fn icon_raw<'a>(glyph: String) -> Text<'a> {
    text(glyph).font(Font::with_name(ICON_FONT))
}

/// Renders `glyph` in the icon font at `size` pixels.
pub fn icon_raw_sized<'a>(glyph: String, size: Option<f32>) -> Text<'a> {
    sized(icon_raw(glyph), size)
}

/// Applies `size` when one is known, leaving the default in place otherwise.
fn sized<'a>(text: Text<'a>, size: Option<f32>) -> Text<'a> {
    match size {
        Some(size) => text.size(size),
        None => text
    }
}
