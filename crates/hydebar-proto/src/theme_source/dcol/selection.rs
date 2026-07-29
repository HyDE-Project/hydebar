//! Which palette applies, and whether it is read backwards.
//!
//! HyDE does not simply use the colours of the current wallpaper. `staterc`
//! carries an `enableWallDcol` switch that decides whether the wallpaper wins
//! over a palette the theme ships, and whether the palette is mirrored so a
//! dark theme keeps its dark surfaces under a light wallpaper. The bar has to
//! reach the same verdict as the scripts do, or it would be the one element on
//! screen coloured the other way round.
//!
//! The rules restated here are the ones `color.set.sh` applies before it
//! renders any template.

use super::palette::DcolMode;

/// Value of `enableWallDcol` when the key is missing.
///
/// The scripts read it as `${enableWallDcol:-0}`, so an install that never set
/// it behaves as if wallpaper recolouring were off.
const DEFAULT_RECOLOUR: u8 = 0;

/// Key the wallpaper recolouring switch is recorded under.
pub(super) const RECOLOUR_KEY: &str = "enableWallDcol";

/// How HyDE was told to recolour the desktop.
///
/// Four settings rather than a flag, because two of them mean "recolour, but
/// only when the wallpaper disagrees with the theme", which a boolean cannot
/// express.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Recolour {
    /// `0` — the theme's own palette wins over the wallpaper.
    ThemeWins,
    /// `1` — the wallpaper palette is used as extracted.
    Wallpaper,
    /// `2` — the wallpaper palette is used, mirrored when it read light.
    WallpaperForcedDark,
    /// `3` — the wallpaper palette is used, mirrored when it read dark.
    WallpaperForcedLight,
    /// Anything else HyDE may grow: treated as plain wallpaper recolouring.
    Unknown
}

impl Recolour {
    /// Reads the setting out of the contents of `staterc`.
    #[must_use]
    pub(super) fn parse(staterc: &str) -> Self {
        match crate::shell_vars::number::<u8>(staterc, RECOLOUR_KEY).unwrap_or(DEFAULT_RECOLOUR) {
            0 => Self::ThemeWins,
            1 => Self::Wallpaper,
            2 => Self::WallpaperForcedDark,
            3 => Self::WallpaperForcedLight,
            _ => Self::Unknown
        }
    }

    /// Whether a palette the theme ships replaces the wallpaper one.
    ///
    /// Only the `0` setting does: with recolouring on, a `theme.dcol` on disk
    /// is ignored, which is why removing that file is the documented way to
    /// get the wallpaper colours back.
    #[must_use]
    pub(super) fn prefers_theme_palette(self) -> bool {
        self == Self::ThemeWins
    }
}

/// Whether the palette has to be mirrored before the bar is painted from it.
///
/// `color_scheme` is the theme's `$COLOR_SCHEME`, as `prefer-dark` or
/// `prefer-light`; it only matters when the theme owns the palette, because
/// that is the one case where the two can disagree about which way round the
/// desktop is meant to look.
///
/// A theme that states nothing is taken at its word rather than inverted: an
/// install with no `$COLOR_SCHEME` anywhere is far more likely to be incomplete
/// than to want its colours flipped.
#[must_use]
pub(super) fn inverts(recolour: Recolour, mode: DcolMode, color_scheme: Option<&str>) -> bool {
    match recolour {
        Recolour::ThemeWins => {
            color_scheme.is_some_and(|scheme| !scheme.to_ascii_lowercase().contains(mode.word()))
        }
        Recolour::WallpaperForcedDark => mode == DcolMode::Light,
        Recolour::WallpaperForcedLight => mode == DcolMode::Dark,
        Recolour::Wallpaper | Recolour::Unknown => false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recolouring_switch_is_read_as_the_scripts_read_it() {
        assert_eq!(Recolour::parse("enableWallDcol=\"0\""), Recolour::ThemeWins);
        assert_eq!(Recolour::parse("enableWallDcol=\"1\""), Recolour::Wallpaper);
        assert_eq!(
            Recolour::parse("enableWallDcol=\"2\""),
            Recolour::WallpaperForcedDark
        );
        assert_eq!(
            Recolour::parse("enableWallDcol=\"3\""),
            Recolour::WallpaperForcedLight
        );
    }

    #[test]
    fn a_missing_switch_leaves_the_theme_palette_in_charge() {
        assert_eq!(Recolour::parse(""), Recolour::ThemeWins);
        assert!(Recolour::parse("").prefers_theme_palette());
    }

    #[test]
    fn only_the_off_setting_lets_a_theme_ship_its_own_palette() {
        assert!(Recolour::ThemeWins.prefers_theme_palette());
        assert!(!Recolour::Wallpaper.prefers_theme_palette());
        assert!(!Recolour::WallpaperForcedDark.prefers_theme_palette());
        assert!(!Recolour::WallpaperForcedLight.prefers_theme_palette());
    }

    #[test]
    fn a_theme_palette_is_mirrored_when_it_disagrees_with_the_theme_preference() {
        assert!(inverts(
            Recolour::ThemeWins,
            DcolMode::Dark,
            Some("prefer-light")
        ));
        assert!(!inverts(
            Recolour::ThemeWins,
            DcolMode::Dark,
            Some("prefer-dark")
        ));
    }

    #[test]
    fn a_theme_that_states_no_preference_is_left_alone() {
        assert!(!inverts(Recolour::ThemeWins, DcolMode::Dark, None));
        assert!(!inverts(Recolour::ThemeWins, DcolMode::Light, None));
    }

    #[test]
    fn a_forced_setting_mirrors_only_the_mode_it_disagrees_with() {
        assert!(inverts(
            Recolour::WallpaperForcedDark,
            DcolMode::Light,
            None
        ));
        assert!(!inverts(
            Recolour::WallpaperForcedDark,
            DcolMode::Dark,
            None
        ));
        assert!(inverts(
            Recolour::WallpaperForcedLight,
            DcolMode::Dark,
            None
        ));
        assert!(!inverts(
            Recolour::WallpaperForcedLight,
            DcolMode::Light,
            None
        ));
    }

    #[test]
    fn plain_wallpaper_recolouring_never_mirrors() {
        assert!(!inverts(
            Recolour::Wallpaper,
            DcolMode::Dark,
            Some("prefer-light")
        ));
        assert!(!inverts(Recolour::Wallpaper, DcolMode::Light, None));
    }
}
