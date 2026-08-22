//! Derivation of the bar's [`Theme`] from the configured appearance.
//!
//! The user writes at most a base and a few optional shades per colour; this
//! is where the full extended palette is generated from them, with every
//! missing shade derived the way `iced` would and every written one honoured
//! verbatim.

use iced::{
    Color, Theme,
    theme::{Palette, palette}
};

mod shades;

use shades::{
    build_danger_pair, build_pair, build_primary_pair, build_secondary_pair, build_success_pair
};

use crate::{config::Appearance, style::color::painted};

/// Builds the `HyDEbar` [`Theme`] from the configured [`Appearance`].
///
/// # Parameters
/// - `appearance`: The appearance configuration provided by the user.
///
/// # Returns
/// A [`Theme`] with palette colours derived from the appearance configuration.
#[must_use]
pub fn hydebar_theme(appearance: &Appearance) -> Theme {
    Theme::custom_with_fn(
        "local".to_string(),
        Palette {
            background: painted(appearance.background_color.get_base()),
            text:       painted(appearance.text_color.get_base()),
            primary:    painted(appearance.primary_color.get_base()),
            success:    painted(appearance.success_color.get_base()),
            warning:    painted(appearance.warning_color.get_base()),
            danger:     painted(appearance.danger_color.get_base())
        },
        |palette| build_extended_palette(appearance, palette)
    )
}

fn build_extended_palette(appearance: &Appearance, palette: Palette) -> palette::Extended {
    let default_bg = palette::Background::new(
        palette.background,
        appearance
            .background_color
            .get_text()
            .map_or(palette.text, painted)
    );
    let default_primary = palette::Primary::generate(
        palette.primary,
        palette.background,
        appearance
            .primary_color
            .get_text()
            .map_or(palette.text, painted)
    );
    let default_secondary = palette::Primary::generate(
        painted(appearance.secondary_color.get_base()),
        palette.background,
        appearance
            .secondary_color
            .get_text()
            .map_or(palette.text, painted)
    );
    let default_success = palette::Success::generate(
        palette.success,
        palette.background,
        appearance
            .success_color
            .get_text()
            .map_or(palette.text, painted)
    );
    let default_danger = palette::Danger::generate(
        palette.danger,
        palette.background,
        appearance
            .danger_color
            .get_text()
            .map_or(palette.text, painted)
    );

    palette::Extended {
        background: build_pair(
            &appearance.background_color,
            palette.text,
            default_bg.base,
            default_bg.weak,
            default_bg.strong
        ),
        primary:    build_primary_pair(&appearance.primary_color, palette.text, default_primary),
        secondary:  build_secondary_pair(
            &appearance.secondary_color,
            palette.text,
            default_secondary
        ),
        success:    build_success_pair(&appearance.success_color, palette.text, default_success),
        warning:    palette::Warning::generate(
            painted(appearance.warning_color.get_base()),
            palette.background,
            appearance
                .warning_color
                .get_text()
                .map_or(palette.text, painted)
        ),
        danger:     build_danger_pair(&appearance.danger_color, palette.text, default_danger),
        is_dark:    palette_is_dark(palette.background)
    }
}

/// Whether `background` reads as a dark surface.
///
/// The extended palette derives its hover and disabled shades towards or away
/// from white depending on this answer; stating "dark" for every theme hands a
/// light `HyDE` palette shades derived the wrong way round.
fn palette_is_dark(background: Color) -> bool {
    0.0722f32.mul_add(
        background.b,
        0.7152f32.mul_add(background.g, 0.2126 * background.r)
    ) < 0.5
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hex_color::HexColor;
    use iced::Color;

    use super::*;
    use crate::config::{Appearance, AppearanceColor, AppearanceStyle};

    #[test]
    fn hydebar_theme_respects_custom_palette() {
        let appearance = Appearance {
            background_color: AppearanceColor::Complete {
                base:   HexColor::rgb(10, 20, 30),
                strong: Some(HexColor::rgb(40, 50, 60)),
                weak:   Some(HexColor::rgb(70, 80, 90)),
                text:   Some(HexColor::rgb(200, 210, 220))
            },
            primary_color: AppearanceColor::Complete {
                base:   HexColor::rgb(120, 60, 30),
                strong: Some(HexColor::rgb(160, 90, 45)),
                weak:   Some(HexColor::rgb(100, 50, 25)),
                text:   Some(HexColor::rgb(255, 255, 255))
            },
            secondary_color: AppearanceColor::Complete {
                base:   HexColor::rgb(15, 25, 35),
                strong: Some(HexColor::rgb(45, 55, 65)),
                weak:   Some(HexColor::rgb(75, 85, 95)),
                text:   None
            },
            success_color: AppearanceColor::Complete {
                base:   HexColor::rgb(20, 120, 20),
                strong: Some(HexColor::rgb(30, 140, 30)),
                weak:   Some(HexColor::rgb(10, 80, 10)),
                text:   Some(HexColor::rgb(0, 0, 0))
            },
            danger_color: AppearanceColor::Complete {
                base:   HexColor::rgb(180, 20, 20),
                strong: Some(HexColor::rgb(200, 40, 40)),
                weak:   Some(HexColor::rgb(160, 10, 10)),
                text:   Some(HexColor::rgb(250, 250, 250))
            },
            text_color: AppearanceColor::Simple(HexColor::rgb(250, 250, 250)),
            style: AppearanceStyle::Islands,
            ..Appearance::default()
        };

        let theme = hydebar_theme(&appearance);
        let palette = theme.extended_palette();

        assert_eq!(palette.background.base.color, Color::from_rgb8(10, 20, 30));
        assert_eq!(palette.background.weak.color, Color::from_rgb8(70, 80, 90));
        assert_eq!(
            palette.background.strong.color,
            Color::from_rgb8(40, 50, 60)
        );
        assert_eq!(palette.primary.base.color, Color::from_rgb8(120, 60, 30));
        assert_eq!(palette.primary.strong.color, Color::from_rgb8(160, 90, 45));
        assert_eq!(palette.primary.base.text, Color::from_rgb8(255, 255, 255));
        assert_eq!(palette.success.weak.color, Color::from_rgb8(10, 80, 10));
        assert_eq!(palette.danger.strong.color, Color::from_rgb8(200, 40, 40));
        assert!(palette.is_dark);
    }

    /// A light `HyDE` theme hands the bar a light island; the derived shades
    /// have to follow it instead of staying on the dark side for ever.
    #[test]
    fn a_light_background_yields_a_light_palette() {
        let appearance = Appearance {
            background_color: AppearanceColor::Simple(HexColor::rgb(235, 235, 230)),
            ..Appearance::default()
        };

        let theme = hydebar_theme(&appearance);

        assert!(!theme.extended_palette().is_dark);
    }
}
