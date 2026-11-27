//! The theme snapshot: what the bar needs from the desktop theme.
//!
//! The snapshot is pure data; [`super::load`] is what assembles it from the
//! desktop.

use crate::theme_source::color::Rgba;

/// Snapshot of the desktop theme as the bar needs it.
///
/// Every field is independent and optional. [`None`] means "nothing on this
/// machine answers for it", which is what lets the bar's own configuration and
/// then its built-in defaults take over field by field instead of all at once.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HydeTheme {
    /// Background of the bar surface.
    pub bar_background:    Option<Rgba>,
    /// Background of an individual island.
    pub module_background: Option<Rgba>,
    /// Default foreground colour.
    pub text:              Option<Rgba>,
    /// Background of an active island.
    pub active_background: Option<Rgba>,
    /// Foreground of an active island.
    pub active_text:       Option<Rgba>,
    /// Background of a hovered island.
    pub hover_background:  Option<Rgba>,
    /// Foreground of a hovered island.
    pub hover_text:        Option<Rgba>,
    /// Font family every widget renders with.
    pub font_family:       Option<String>,
    /// Font size in pixels.
    pub font_size_px:      Option<f32>,
    /// Corner radius of a full island, in pixels.
    pub radius_px:         Option<f32>
}

impl HydeTheme {
    /// Fills every field this snapshot leaves unset from `fallback`.
    ///
    /// Merging rather than replacing is what keeps the precedence honest: a
    /// value already read from a `HyDE` directory is never displaced by one
    /// guessed from a generated stylesheet.
    pub(super) fn fill_gaps_from(&mut self, fallback: Self) {
        self.bar_background = self.bar_background.or(fallback.bar_background);
        self.module_background = self.module_background.or(fallback.module_background);
        self.text = self.text.or(fallback.text);
        self.active_background = self.active_background.or(fallback.active_background);
        self.active_text = self.active_text.or(fallback.active_text);
        self.hover_background = self.hover_background.or(fallback.hover_background);
        self.hover_text = self.hover_text.or(fallback.hover_text);

        if self.font_family.is_none() {
            self.font_family = fallback.font_family;
        }

        self.font_size_px = self.font_size_px.or(fallback.font_size_px);
        self.radius_px = self.radius_px.or(fallback.radius_px);
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_snapshot_only_takes_from_the_fallback_what_it_lacks() {
        let mut theme = HydeTheme {
            text: Some(Rgba::rgb(1, 2, 3)),
            ..HydeTheme::default()
        };

        theme.fill_gaps_from(HydeTheme {
            text: Some(Rgba::rgb(9, 9, 9)),
            radius_px: Some(4.0),
            font_family: Some(String::from("Fira Sans")),
            ..HydeTheme::default()
        });

        assert_eq!(theme.text, Some(Rgba::rgb(1, 2, 3)));
        assert_eq!(theme.radius_px, Some(4.0));
        assert_eq!(theme.font_family.as_deref(), Some("Fira Sans"));
    }
}
