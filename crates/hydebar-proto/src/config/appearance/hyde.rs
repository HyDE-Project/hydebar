//! Overlay of the theme published by the HyDE Project onto the appearance
//! configuration.

use hex_color::HexColor;
use iced::Color;

use super::{
    color::{
        AppearanceColor, default_background_color, default_primary_color, default_text_color,
        default_workspace_colors
    },
    settings::{Appearance, default_bar_opacity, default_opacity}
};
use crate::theme_source::{HydeTheme, Rgba};

impl Appearance {
    /// Fills every field the user left at its default with the HyDE theme.
    ///
    /// Explicit configuration always wins: a field is only overlaid when it
    /// still holds the value it would have without a configuration file. Values
    /// the theme does not provide are left untouched.
    ///
    /// The alpha channel of the HyDE colors carries the transparency: the
    /// module background feeds [`Appearance::opacity`] and the bar background
    /// feeds [`Appearance::bar_opacity`]. Every other color is translucent over
    /// the island it is painted on, so it is composited over the island
    /// background instead of losing its alpha.
    pub fn apply_hyde_theme(&mut self, theme: &HydeTheme) {
        if self.font_name.is_none() {
            self.font_name.clone_from(&theme.font_family);
        }

        if self.font_size.is_none() {
            self.font_size = theme.font_size_px;
        }

        if self.radius.is_none() {
            self.radius = theme.radius_px;
        }

        if let Some(module_background) = theme.module_background {
            if self.background_color == default_background_color() {
                let island = hex_to_color(opaque_hex(module_background));

                self.background_color = AppearanceColor::Complete {
                    base:   opaque_hex(module_background),
                    strong: None,
                    weak:   theme.hover_background.map(|hover| blend_hex(hover, island)),
                    text:   None
                };
            }

            if self.opacity == default_opacity() {
                self.opacity = module_background.a;
            }
        }

        let island = self.background_color.get_base();

        if let Some(bar_background) = theme.bar_background
            && self.bar_opacity == default_bar_opacity()
        {
            self.bar_opacity = bar_background.a;
        }

        if let Some(text) = theme.text
            && self.text_color == default_text_color()
        {
            self.text_color = AppearanceColor::Simple(blend_hex(text, island));
        }

        if let Some(active_background) = theme.active_background
            && self.primary_color == default_primary_color()
        {
            self.primary_color = AppearanceColor::Complete {
                base:   blend_hex(active_background, island),
                strong: None,
                weak:   None,
                text:   theme
                    .active_text
                    .map(|text| blend_hex(text, island))
                    .or_else(|| text_hex(default_primary_color()))
            };
        }

        if let Some(active_background) = theme.active_background
            && self.workspace_colors == default_workspace_colors()
        {
            self.workspace_colors = vec![AppearanceColor::Complete {
                base:   blend_hex(active_background, island),
                strong: None,
                weak:   None,
                text:   theme.active_text.map(|text| blend_hex(text, island))
            }];
        }
    }
}

/// Drops the alpha channel of a HyDE color, which the opacity fields carry
/// instead.
fn opaque_hex(color: Rgba) -> HexColor {
    HexColor::rgb(color.r, color.g, color.b)
}

/// Reads a palette entry back as a [`Color`].
fn hex_to_color(color: HexColor) -> Color {
    Color::from_rgb8(color.r, color.g, color.b)
}

/// Composites a translucent HyDE color over the surface it is painted on.
///
/// The stylesheets state their accents with an alpha channel: the focused
/// workspace is `rgba(195,172,118,0.4)`, a muted tint of the island rather than
/// the saturated color its channels name. Dropping the alpha the way
/// [`opaque_hex`] does would repaint the bar with that saturated shade, so the
/// value is blended over `backdrop` first, which is the island the widget sits
/// on. The island keeps its own translucency through [`Appearance::opacity`];
/// only the tint is resolved here.
fn blend_hex(color: Rgba, backdrop: Color) -> HexColor {
    let alpha = color.a.clamp(0.0, 1.0);
    let mix = |channel: u8, under: f32| -> u8 {
        let over = f32::from(channel) / 255.0;
        let blended = over.mul_add(alpha, under * (1.0 - alpha));

        (blended * 255.0).round().clamp(0.0, 255.0) as u8
    };

    HexColor::rgb(
        mix(color.r, backdrop.r),
        mix(color.g, backdrop.g),
        mix(color.b, backdrop.b)
    )
}

/// Returns the text shade of a palette entry, if it declares one.
fn text_hex(color: AppearanceColor) -> Option<HexColor> {
    match color {
        AppearanceColor::Simple(_) => None,
        AppearanceColor::Complete {
            text, ..
        } => text
    }
}

#[cfg(test)]
mod tests {
    use super::{super::metrics::MODULE_GAP_EM, *};

    fn hyde_theme() -> HydeTheme {
        HydeTheme {
            bar_background:    Some(Rgba::rgba(1, 2, 3, 0.25)),
            module_background: Some(Rgba::rgba(27, 29, 28, 0.8)),
            text:              Some(Rgba::rgb(170, 240, 205)),
            active_background: Some(Rgba::rgb(195, 172, 118)),
            active_text:       Some(Rgba::rgb(255, 240, 204)),
            hover_background:  Some(Rgba::rgba(125, 108, 75, 0.4)),
            hover_text:        None,
            font_family:       Some(String::from("JetBrainsMono Nerd Font")),
            font_size_px:      Some(10.0),
            radius_px:         Some(4.0)
        }
    }

    #[test]
    fn hyde_theme_fills_unset_values() {
        let mut appearance = Appearance::default();
        appearance.apply_hyde_theme(&hyde_theme());

        assert_eq!(
            appearance.font_name.as_deref(),
            Some("JetBrainsMono Nerd Font")
        );
        assert_eq!(appearance.font_size, Some(10.0));
        assert_eq!(appearance.radius, Some(4.0));
        assert_eq!(appearance.pill_radius(), 4.0);
        assert_eq!(appearance.opacity, 0.8);
        assert_eq!(appearance.bar_opacity, 0.25);
        assert_eq!(
            appearance.background_color,
            AppearanceColor::Complete {
                base:   HexColor::rgb(27, 29, 28),
                strong: None,
                weak:   Some(HexColor::rgb(66, 61, 47)),
                text:   None
            }
        );
        assert_eq!(
            appearance.text_color,
            AppearanceColor::Simple(HexColor::rgb(170, 240, 205))
        );
        assert_eq!(
            appearance.primary_color,
            AppearanceColor::Complete {
                base:   HexColor::rgb(195, 172, 118),
                strong: None,
                weak:   None,
                text:   Some(HexColor::rgb(255, 240, 204))
            }
        );
    }

    /// Mirrors the colors the HyDE stylesheets ship, alpha channels included.
    fn translucent_hyde_theme() -> HydeTheme {
        HydeTheme {
            module_background: Some(Rgba::rgba(27, 29, 28, 0.8)),
            text: Some(Rgba::rgba(170, 240, 205, 0.8)),
            active_background: Some(Rgba::rgba(195, 172, 118, 0.4)),
            active_text: Some(Rgba::rgba(255, 240, 204, 1.0)),
            hover_background: Some(Rgba::rgba(125, 108, 75, 0.4)),
            ..hyde_theme()
        }
    }

    #[test]
    fn translucent_theme_colors_are_composited_over_the_island() {
        let mut appearance = Appearance::default();
        appearance.apply_hyde_theme(&translucent_hyde_theme());

        assert_eq!(appearance.opacity, 0.8);
        assert_eq!(
            appearance.text_color,
            AppearanceColor::Simple(HexColor::rgb(141, 198, 170))
        );
        assert_eq!(
            appearance.primary_color,
            AppearanceColor::Complete {
                base:   HexColor::rgb(94, 86, 64),
                strong: None,
                weak:   None,
                text:   Some(HexColor::rgb(255, 240, 204))
            }
        );
        assert_eq!(
            appearance.workspace_colors,
            vec![AppearanceColor::Complete {
                base:   HexColor::rgb(94, 86, 64),
                strong: None,
                weak:   None,
                text:   Some(HexColor::rgb(255, 240, 204))
            }]
        );
        assert_eq!(
            appearance.background_color,
            AppearanceColor::Complete {
                base:   HexColor::rgb(27, 29, 28),
                strong: None,
                weak:   Some(HexColor::rgb(66, 61, 47)),
                text:   None
            }
        );
    }

    #[test]
    fn an_opaque_theme_color_survives_compositing_unchanged() {
        assert_eq!(
            blend_hex(Rgba::rgb(195, 172, 118), Color::from_rgb8(27, 29, 28)),
            HexColor::rgb(195, 172, 118)
        );
        assert_eq!(
            blend_hex(Rgba::rgba(195, 172, 118, 0.0), Color::from_rgb8(27, 29, 28)),
            HexColor::rgb(27, 29, 28)
        );
    }

    #[test]
    fn hyde_theme_leaves_user_set_values_untouched() {
        let configured = Appearance {
            font_name: Some(String::from("Fira Sans")),
            font_size: Some(14.0),
            radius: Some(0.0),
            opacity: 0.5,
            bar_opacity: 0.9,
            background_color: AppearanceColor::Simple(HexColor::rgb(1, 1, 1)),
            text_color: AppearanceColor::Simple(HexColor::rgb(2, 2, 2)),
            primary_color: AppearanceColor::Simple(HexColor::rgb(3, 3, 3)),
            workspace_colors: vec![AppearanceColor::Simple(HexColor::rgb(4, 4, 4))],
            ..Appearance::default()
        };

        let mut appearance = configured.clone();
        appearance.apply_hyde_theme(&hyde_theme());

        assert_eq!(appearance, configured);
    }

    #[test]
    fn an_empty_hyde_theme_changes_nothing() {
        let mut appearance = Appearance::default();
        appearance.apply_hyde_theme(&HydeTheme::default());

        assert_eq!(appearance, Appearance::default());
    }

    #[test]
    fn a_themed_font_size_changes_the_derived_spacing() {
        let mut appearance = Appearance::default();
        let fallback_gap = appearance.module_gap();
        let fallback_padding = appearance.module_padding();

        appearance.apply_hyde_theme(&hyde_theme());

        assert_eq!(appearance.font_size, Some(10.0));
        assert_ne!(appearance.module_gap(), fallback_gap);
        assert_ne!(appearance.module_padding(), fallback_padding);
        assert_eq!(appearance.module_gap(), 10.0 * MODULE_GAP_EM);
        assert_eq!(appearance.bar_padding(), [3.0, 16.0]);
    }
}
