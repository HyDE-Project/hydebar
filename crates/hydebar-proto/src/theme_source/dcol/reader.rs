//! Reading the palette that is in force out of a `HyDE` install.

use std::fs;

use super::{
    palette::DcolPalette,
    roles::BarColors,
    selection::{Recolour, inverts}
};
use crate::{hyde_dirs::HydeDirs, hypr_vars, shell_vars};

/// Key the active theme is recorded under in `staterc`.
const THEME_KEY: &str = "HYDE_THEME";

/// Name the light/dark preference is stated under.
///
/// Spelled the same way as a hyprlang variable and as a shell assignment, which
/// is why one constant covers both files it is read from.
const COLOR_SCHEME: &str = "COLOR_SCHEME";

/// Reads the palette the desktop is currently coloured from.
///
/// Everything comes out of the `HyDE` directories and nothing else:
/// `~/.local/state/hyde/staterc` names the theme and the recolouring mode,
/// `~/.cache/hyde/wall.dcol` carries the wallpaper colours and
/// `~/.config/hyde/themes/<theme>/theme.dcol` the ones a theme ships. No
/// generated stylesheet is consulted, so the result stays correct whatever else
/// on the desktop is or is not running.
///
/// Returns [`None`] when no palette can be read whole, which is the signal to
/// fall back to another source rather than paint the bar half-themed.
#[must_use]
pub(in crate::theme_source) fn read(dirs: &HydeDirs) -> Option<BarColors> {
    Some(BarColors::from_palette(&read_palette(dirs)?))
}

/// Reads the palette in force, mirrored if the theme asks for it.
fn read_palette(dirs: &HydeDirs) -> Option<DcolPalette> {
    let staterc = fs::read_to_string(dirs.staterc()).unwrap_or_default();
    let recolour = Recolour::parse(&staterc);
    let theme = shell_vars::value_of(&staterc, THEME_KEY);

    let palette = DcolPalette::parse(&read_source(dirs, theme.as_deref(), recolour)?)?;

    if inverts(
        recolour,
        palette.mode,
        color_scheme(dirs, theme.as_deref()).as_deref()
    ) {
        return Some(palette.inverted());
    }

    Some(palette)
}

/// Reads the `.dcol` file that applies, preferring a theme's own palette when
/// the recolouring switch says it wins.
///
/// The wallpaper palette is read through `~/.cache/hyde/wall.dcol`, a symlink
/// `HyDE` re-points on every wallpaper change, so following it is what makes the
/// bar see the same colours the rest of the desktop was given.
fn read_source(dirs: &HydeDirs, theme: Option<&str>, recolour: Recolour) -> Option<String> {
    if recolour.prefers_theme_palette()
        && let Some(theme) = theme
        && let Ok(source) = fs::read_to_string(dirs.theme_dcol(theme))
    {
        return Some(source);
    }

    fs::read_to_string(dirs.wall_dcol()).ok()
}

/// Resolves the light/dark preference that is in force.
///
/// Follows the order `HyDE` resolves a theme variable in: the user's own Hyprland
/// override outranks the theme fragment, which outranks the values `HyDE` ships.
/// Reading all three matters because the preference is what decides whether the
/// palette is mirrored, and a wrong answer inverts the whole bar.
fn color_scheme(dirs: &HydeDirs, theme: Option<&str>) -> Option<String> {
    let override_source = fs::read_to_string(dirs.hyprland_override()).unwrap_or_default();

    if let Some(scheme) = hypr_vars::value_of(&override_source, COLOR_SCHEME) {
        return Some(scheme);
    }

    if let Some(theme) = theme
        && let Ok(source) = fs::read_to_string(dirs.hypr_theme(theme))
        && let Some(scheme) = hypr_vars::value_of(&source, COLOR_SCHEME)
    {
        return Some(scheme);
    }

    let shipped = fs::read_to_string(dirs.env_theme()).unwrap_or_default();

    shell_vars::value_of(&shipped, COLOR_SCHEME)
}

#[cfg(test)]
mod tests {
    use super::{
        super::fixtures::{THEME_DCOL, WALL_DCOL},
        *
    };
    use crate::theme_source::color::Rgba;

    /// A `HyDE` install laid out in a temporary directory.
    struct Install {
        _root: tempfile::TempDir,
        dirs:  HydeDirs
    }

    impl Install {
        fn new() -> Self {
            let root = tempfile::TempDir::new().expect("tempdir");
            let dirs = HydeDirs::new(
                root.path().join("config"),
                root.path().join("state"),
                root.path().join("cache"),
                root.path().join("data")
            );

            Self {
                _root: root,
                dirs
            }
        }

        fn write(path: std::path::PathBuf, contents: &str) {
            fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
            fs::write(path, contents).expect("write fixture");
        }

        fn with_staterc(self, contents: &str) -> Self {
            Self::write(self.dirs.staterc(), contents);
            self
        }

        fn with_wall_dcol(self, contents: &str) -> Self {
            Self::write(self.dirs.wall_dcol(), contents);
            self
        }

        fn with_theme_dcol(self, theme: &str, contents: &str) -> Self {
            Self::write(self.dirs.theme_dcol(theme), contents);
            self
        }

        fn with_hypr_theme(self, theme: &str, contents: &str) -> Self {
            Self::write(self.dirs.hypr_theme(theme), contents);
            self
        }
    }

    #[test]
    fn the_wallpaper_palette_colours_the_bar_while_recolouring_is_on() {
        let install = Install::new()
            .with_staterc("HYDE_THEME=\"Fixture\"\nenableWallDcol=\"1\"\n")
            .with_wall_dcol(WALL_DCOL);

        let colors = read(&install.dirs).expect("colors");

        assert_eq!(colors.module_background, Rgba::rgba(72, 56, 40, 0.8));
    }

    #[test]
    fn a_theme_palette_wins_while_recolouring_is_off() {
        let install = Install::new()
            .with_staterc("HYDE_THEME=\"Fixture\"\nenableWallDcol=\"0\"\n")
            .with_wall_dcol(WALL_DCOL)
            .with_theme_dcol("Fixture", THEME_DCOL)
            .with_hypr_theme("Fixture", "$COLOR_SCHEME = prefer-light\n");

        let colors = read(&install.dirs).expect("colors");

        assert_eq!(colors.module_background, Rgba::rgba(132, 56, 40, 0.8));
    }

    #[test]
    fn a_theme_palette_is_ignored_while_recolouring_is_on() {
        let install = Install::new()
            .with_staterc("HYDE_THEME=\"Fixture\"\nenableWallDcol=\"1\"\n")
            .with_wall_dcol(WALL_DCOL)
            .with_theme_dcol("Fixture", THEME_DCOL);

        let colors = read(&install.dirs).expect("colors");

        assert_eq!(colors.module_background, Rgba::rgba(72, 56, 40, 0.8));
    }

    #[test]
    fn a_theme_disagreeing_with_its_palette_is_painted_mirrored() {
        let install = Install::new()
            .with_staterc("HYDE_THEME=\"Fixture\"\nenableWallDcol=\"0\"\n")
            .with_wall_dcol(WALL_DCOL)
            .with_hypr_theme("Fixture", "$COLOR_SCHEME = prefer-light\n");

        let colors = read(&install.dirs).expect("colors");

        assert_eq!(colors.module_background, Rgba::rgba(4, 3, 2, 0.8));
    }

    #[test]
    fn a_hyprland_override_outranks_the_theme_preference() {
        let install = Install::new()
            .with_staterc("HYDE_THEME=\"Fixture\"\nenableWallDcol=\"0\"\n")
            .with_wall_dcol(WALL_DCOL)
            .with_hypr_theme("Fixture", "$COLOR_SCHEME = prefer-light\n");
        Install::write(
            install.dirs.hyprland_override(),
            "$COLOR_SCHEME = prefer-dark\n"
        );

        let colors = read(&install.dirs).expect("colors");

        assert_eq!(colors.module_background, Rgba::rgba(72, 56, 40, 0.8));
    }

    #[test]
    fn an_install_without_a_palette_reads_as_nothing() {
        assert_eq!(read(&Install::new().dirs), None);
    }

    #[test]
    fn a_truncated_palette_reads_as_nothing() {
        let install = Install::new()
            .with_staterc("enableWallDcol=\"1\"\n")
            .with_wall_dcol("dcol_mode=\"dark\"\ndcol_pry1=\"483828\"\n");

        assert_eq!(read(&install.dirs), None);
    }
}
