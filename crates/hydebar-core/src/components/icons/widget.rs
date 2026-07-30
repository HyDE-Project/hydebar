//! Text widgets rendering a glyph in the icon font.

use iced::widget::Text;

use super::{super::text::text, catalog::Icons, theme::IconTheme};

/// Renders `icon` at the size the table carries.
pub fn icon<'a>(theme: &IconTheme, icon: Icons) -> Text<'a> {
    sized(icon_raw(theme.glyph(icon).to_owned()), theme.size())
}

/// Renders an arbitrary glyph, at the themed size.
///
/// No font is named here on purpose: the glyph renders in the themed font and
/// falls back through the system font database when that font lacks it — the
/// same road the reference waybar theme takes, where no icon font is declared
/// anywhere and the symbols resolve to whatever nerd font the system carries.
///
/// Built on the themed text helper for the size: a glyph left at the renderer
/// default would stay small while the rest of the bar follows the screen.
pub fn icon_raw<'a>(glyph: String) -> Text<'a> {
    text(glyph)
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
