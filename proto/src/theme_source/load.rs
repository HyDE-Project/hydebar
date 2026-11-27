//! Assembling the theme snapshot from the desktop's own sources.
//!
//! Reads the desktop theme out of a `HyDE` install and the running compositor.
//!
//! Sources, in the order they are preferred:
//!
//! 1. the `HyDE` directories — `~/.cache/hyde` for the colours,
//!    `~/.config/hyde` and `~/.local/state/hyde` for the font;
//! 2. the compositor, for the corner radius, because the bar should round its
//!    islands exactly as the windows beside them are rounded;
//! 3. the stylesheets under `~/.config/waybar`, and only as a last resort — see
//!    [`super::waybar`] for why they cannot be trusted.
//!
//! Nothing here outranks the bar's own configuration file: the caller overlays
//! this snapshot onto the configuration and only fills what the user left
//! unset.

use super::{dcol, font, theme::HydeTheme, waybar};
use crate::{compositor_look::CompositorLook, hyde_dirs::HydeDirs};

/// Reads the desktop theme out of a `HyDE` install and the running compositor.
///
/// `look` is taken rather than read so a caller can decide whether querying the
/// compositor is worth it, and so a test can describe one without a session.
#[must_use]
pub fn load_from(dirs: &HydeDirs, look: &CompositorLook) -> HydeTheme {
    let mut theme = HydeTheme::default();

    if dirs.is_installed() {
        apply_hyde(&mut theme, dirs);
    }

    theme.radius_px = theme.radius_px.or(look.rounding);
    theme.fill_gaps_from(waybar::read(&dirs.config));

    theme
}

/// Reads the desktop theme of the running session.
///
/// Returns an empty snapshot when the environment names no home directory, in
/// which case the bar keeps its own configuration and defaults throughout.
#[must_use]
pub fn load() -> HydeTheme {
    HydeDirs::from_env().map_or_else(HydeTheme::default, |dirs| {
        load_from(&dirs, &CompositorLook::read())
    })
}

/// Fills the snapshot from the `HyDE` directories.
///
/// The colours are all-or-nothing: a palette that could not be read whole
/// leaves every colour unset so the fallback can supply a consistent set,
/// rather than mixing half a new theme into the old one.
fn apply_hyde(theme: &mut HydeTheme, dirs: &HydeDirs) {
    if let Some(colors) = dcol::read(dirs) {
        theme.bar_background = Some(colors.bar_background);
        theme.module_background = Some(colors.module_background);
        theme.text = Some(colors.text);
        theme.active_background = Some(colors.active_background);
        theme.active_text = Some(colors.active_text);
        theme.hover_background = Some(colors.hover_background);
        theme.hover_text = Some(colors.hover_text);
    }

    let font = font::read(dirs);

    theme.font_family = Some(font.family);
    theme.font_size_px = Some(font.size_px);
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::theme_source::{
        color::Rgba,
        fixtures::{assert_close, write_fixture}
    };

    /// A palette in the shape wallbash writes it, built from the shared
    /// fixture.
    fn wall_dcol() -> String {
        let mut lines = vec![String::from("dcol_mode=\"dark\"")];

        for group in 1..=4u8 {
            lines.push(format!("dcol_pry{group}=\"48382{group}\""));
            lines.push(format!("dcol_txt{group}=\"FFFFF{group}\""));

            for step in 1..=9u8 {
                lines.push(format!("dcol_{group}xa{step}=\"1{group}2{step}33\""));
            }
        }

        lines.join("\n")
    }

    struct Install {
        _root: TempDir,
        dirs:  HydeDirs
    }

    impl Install {
        fn new() -> Self {
            let root = TempDir::new().expect("tempdir");
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

        fn write(self, path: PathBuf, contents: &str) -> Self {
            fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
            fs::write(path, contents).expect("write fixture");
            self
        }

        fn hyde(self) -> Self {
            let path = self.dirs.staterc();
            self.write(path, "HYDE_THEME=\"Fixture\"\nenableWallDcol=\"1\"\n")
        }

        fn palette(self) -> Self {
            let path = self.dirs.wall_dcol();
            let source = wall_dcol();
            self.write(path, &source)
        }

        fn waybar(self) -> Self {
            fs::create_dir_all(&self.dirs.config).expect("create config directory");
            write_fixture(&self.dirs.config);
            self
        }
    }

    /// A compositor that rounds its windows by six pixels.
    fn look() -> CompositorLook {
        CompositorLook {
            rounding: Some(6.0),
            ..CompositorLook::default()
        }
    }

    #[test]
    fn a_hyde_install_colours_the_bar_from_its_own_cache() {
        let install = Install::new().hyde().palette().waybar();

        let theme = load_from(&install.dirs, &look());

        assert_eq!(theme.module_background, Some(Rgba::rgba(72, 56, 33, 0.8)));
        assert_eq!(theme.text, Some(Rgba::rgba(0x11, 0x28, 0x33, 0.8)));
    }

    #[test]
    fn the_generated_stylesheets_never_displace_a_hyde_palette() {
        let with_waybar = load_from(&Install::new().hyde().palette().waybar().dirs, &look());
        let without_waybar = load_from(&Install::new().hyde().palette().dirs, &look());

        assert_eq!(
            with_waybar.module_background,
            without_waybar.module_background
        );
        assert_eq!(with_waybar.text, without_waybar.text);
        assert_eq!(
            with_waybar.active_background,
            without_waybar.active_background
        );
    }

    #[test]
    fn the_compositor_owns_the_corner_radius_of_a_hyde_desktop() {
        let install = Install::new().hyde().palette().waybar();

        assert_eq!(load_from(&install.dirs, &look()).radius_px, Some(6.0));
    }

    #[test]
    fn a_hyde_install_takes_its_font_from_hyde_and_not_from_a_stylesheet() {
        let install = Install::new().hyde().palette().waybar();
        let install = {
            let path = install.dirs.env_theme();
            install.write(path, "BAR_FONT=\"Iosevka Nerd Font\"\n")
        };

        let theme = load_from(&install.dirs, &CompositorLook::default());

        assert_eq!(theme.font_family.as_deref(), Some("Iosevka Nerd Font"));
        assert_close(theme.font_size_px.expect("font size"), 10.0);
    }

    #[test]
    fn a_hyde_install_without_a_palette_falls_back_to_the_stylesheets() {
        let install = Install::new().hyde().waybar();

        let theme = load_from(&install.dirs, &CompositorLook::default());

        assert_eq!(theme.module_background, Some(Rgba::rgba(27, 29, 28, 0.8)));
        assert_close(theme.radius_px.expect("radius"), 4.0);
    }

    #[test]
    fn a_desktop_without_hyde_is_not_given_the_hyde_defaults() {
        let install = Install::new();

        let theme = load_from(&install.dirs, &CompositorLook::default());

        assert_eq!(theme, HydeTheme::default());
    }

    #[test]
    fn a_desktop_without_hyde_still_reads_whatever_stylesheets_it_has() {
        let install = Install::new().waybar();

        let theme = load_from(&install.dirs, &CompositorLook::default());

        assert_eq!(
            theme.font_family.as_deref(),
            Some("JetBrainsMono Nerd Font")
        );
        assert_eq!(theme.module_background, Some(Rgba::rgba(27, 29, 28, 0.8)));
    }

    #[test]
    fn a_palette_that_cannot_be_read_whole_leaves_every_colour_to_the_fallback() {
        let install = Install::new().hyde().waybar();
        let install = {
            let path = install.dirs.wall_dcol();
            install.write(path, "dcol_mode=\"dark\"\ndcol_pry1=\"483828\"\n")
        };

        let theme = load_from(&install.dirs, &CompositorLook::default());

        assert_eq!(theme.module_background, Some(Rgba::rgba(27, 29, 28, 0.8)));
        assert_eq!(theme.text, Some(Rgba::rgb(170, 240, 205)));
    }
}
