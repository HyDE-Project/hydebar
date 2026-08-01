//! Personality of the palette front a theme switch sends across the bar.
//!
//! Switching to a theme plays that theme's own entrance: the front picks its
//! corner, its width, its pace and its damping from the theme it brings in, so
//! arriving in a neon theme snaps and bounces while a warm retro palette rolls
//! in slowly. The stock `HyDE` themes each carry a hand-tuned signature; any
//! other theme derives a stable one from its own palette, so no theme ever
//! falls back to somebody else's motion.

use std::time::Duration;

use crate::config::Appearance;

/// How the palette of one theme enters the bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepStyle {
    /// Which end of the bar the front starts from.
    pub from_left: bool,
    /// Share of the travel spent staggering the islands, zero to one.
    pub spread:    f32,
    /// Response of the spring carrying the front.
    pub response:  Duration,
    /// Damping of that spring; one never overshoots, lower bounces.
    pub damping:   f32
}

impl Default for SweepStyle {
    fn default() -> Self {
        Self {
            from_left: false,
            spread:    0.75,
            response:  Duration::from_millis(1100),
            damping:   1.0
        }
    }
}

impl SweepStyle {
    /// The signature of `theme`, derived from its palette when it is not a
    /// stock one.
    #[must_use]
    pub fn of(theme: Option<&str>, appearance: &Appearance) -> Self {
        theme
            .and_then(Self::stock)
            .unwrap_or_else(|| Self::derived(theme, appearance))
    }

    /// Hand-tuned signatures of the themes `HyDE` ships.
    fn stock(theme: &str) -> Option<Self> {
        let style = |from_left, spread, millis, damping| {
            Some(Self {
                from_left,
                spread,
                response: Duration::from_millis(millis),
                damping
            })
        };

        match theme {
            "Tokyo Night" => style(false, 0.8, 1000, 0.9),
            "Gruvbox Retro" => style(true, 0.7, 1400, 1.0),
            "Catppuccin Mocha" => style(false, 0.75, 1200, 1.0),
            "Catppuccin Latte" => style(true, 0.7, 900, 1.0),
            "Decay Green" => style(false, 0.8, 1500, 1.0),
            "Edge Runner" => style(false, 0.85, 800, 0.8),
            "Frosted Glass" => style(true, 0.6, 1300, 1.0),
            "Graphite Mono" => style(false, 0.4, 1000, 1.0),
            "Material Sakura" => style(true, 0.7, 1200, 0.95),
            "Nordic Blue" => style(false, 0.7, 1300, 1.0),
            "Rosé Pine" => style(true, 0.75, 1250, 1.0),
            "Synth Wave" => style(false, 0.85, 850, 0.75),
            _ => None
        }
    }

    /// A stable signature for a theme the table does not name.
    ///
    /// The palette decides the character — a dark background rolls in slower
    /// than a light one, a saturated accent moves with more spring — and the
    /// name decides the corner, so the same theme always enters the same way.
    fn derived(theme: Option<&str>, appearance: &Appearance) -> Self {
        let background = appearance.background_color.get_base();
        let luminance = 0.0722f32.mul_add(
            background.b,
            0.7152f32.mul_add(background.g, 0.2126 * background.r)
        );
        let dark = luminance < 0.5;

        let primary = appearance.primary_color.get_base();
        let low = primary.r.min(primary.g).min(primary.b);
        let high = primary.r.max(primary.g).max(primary.b);
        let vivid = high - low > 0.375;

        Self {
            from_left: theme
                .is_some_and(|name| name.bytes().fold(0_u8, u8::wrapping_add) % 2 == 0),
            spread:    0.75,
            response:  Duration::from_millis(if dark { 1300 } else { 950 }),
            damping:   if vivid { 0.85 } else { 1.0 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stock_theme_carries_its_own_signature() {
        let names = [
            "Tokyo Night",
            "Gruvbox Retro",
            "Catppuccin Mocha",
            "Catppuccin Latte",
            "Decay Green",
            "Edge Runner",
            "Frosted Glass",
            "Graphite Mono",
            "Material Sakura",
            "Nordic Blue",
            "Rosé Pine",
            "Synth Wave"
        ];

        for name in names {
            let style = SweepStyle::of(Some(name), &Appearance::default());

            assert!(SweepStyle::stock(name).is_some(), "{name}");
            assert!(style.spread > 0.0 && style.spread < 1.0, "{name}");
            assert!(style.damping > 0.5 && style.damping <= 1.0, "{name}");
            assert!(
                style.response >= Duration::from_millis(700)
                    && style.response <= Duration::from_millis(1600),
                "{name}"
            );
        }
    }

    #[test]
    fn a_neon_theme_enters_differently_from_a_retro_one() {
        let appearance = Appearance::default();
        let neon = SweepStyle::of(Some("Edge Runner"), &appearance);
        let retro = SweepStyle::of(Some("Gruvbox Retro"), &appearance);

        assert_ne!(neon.from_left, retro.from_left);
        assert!(neon.response < retro.response);
        assert!(neon.damping < retro.damping);
    }

    #[test]
    fn an_unknown_theme_derives_a_stable_signature() {
        let appearance = Appearance::default();

        let first = SweepStyle::of(Some("Somebody's Theme"), &appearance);
        let again = SweepStyle::of(Some("Somebody's Theme"), &appearance);

        assert_eq!(first, again);
        assert!(first.spread > 0.0);
    }

    #[test]
    fn a_light_palette_derives_a_quicker_entrance_than_a_dark_one() {
        use hydebar_proto::config::AppearanceColor;

        let dark = Appearance::default();
        let light = Appearance {
            background_color: AppearanceColor::Simple(hex_color::HexColor::rgb(240, 240, 240)),
            ..Appearance::default()
        };

        let slow = SweepStyle::of(Some("Somebody's Theme"), &dark);
        let quick = SweepStyle::of(Some("Somebody's Theme"), &light);

        assert!(quick.response < slow.response);
    }

    #[test]
    fn a_saturated_accent_derives_a_springier_entrance() {
        use hydebar_proto::config::AppearanceColor;

        let vivid = Appearance {
            primary_color: AppearanceColor::Simple(hex_color::HexColor::rgb(255, 40, 160)),
            ..Appearance::default()
        };
        let grey = Appearance {
            primary_color: AppearanceColor::Simple(hex_color::HexColor::rgb(120, 120, 120)),
            ..Appearance::default()
        };

        let muted = SweepStyle::of(Some("Somebody's Theme"), &grey);
        let springy = SweepStyle::of(Some("Somebody's Theme"), &vivid);

        assert!(springy.damping < muted.damping);
        assert!((muted.damping - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn no_theme_at_all_still_yields_a_usable_signature() {
        let style = SweepStyle::of(None, &Appearance::default());

        assert!(style.response > Duration::ZERO);
        assert!(style.spread > 0.0 && style.spread < 1.0);
    }
}
