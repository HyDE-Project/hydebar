//! The font `HyDE` means a bar to use.
//!
//! The font is not part of the palette and is not written to any stylesheet the
//! bar could read: `HyDE` resolves it on the fly, out of four files, each of
//! which may override the one below it. The chain is restated here so the bar
//! can answer the same question without a bar generator having run first.
//!
//! Order, highest first, mirroring what `HyDE` itself does:
//!
//! 1. `~/.local/state/hyde/config` — the flat export the `hyde-config` daemon
//!    writes out of `config.toml`; `~/.config/hyde/config.toml` itself is the
//!    fallback for installs old enough to keep flat assignments there —
//!    `WAYBAR_FONT`, `WAYBAR_SCALE`
//! 2. `~/.config/hyde/themes/<theme>/hypr.theme` — `$BAR_FONT`,
//!    `$BAR_FONT_SIZE`
//! 3. `~/.local/state/hyde/staterc` — `BAR_FONT`, `BAR_FONT_SIZE`
//! 4. `~/.local/share/hyde/env-theme` — the values `HyDE` ships
//! 5. the `HyDE` defaults, `JetBrainsMono Nerd Font` at `10`
//!
//! The size is resolved slightly differently from the family, again mirroring
//! `HyDE`: the session state outranks the theme, because a size is something
//! the user nudges for their screen while a family is something the theme
//! picks.

use crate::{hyde_dirs::HydeDirs, hyde_files, hypr_vars, shell_vars};

/// Family `HyDE` falls back to when nothing names one.
const DEFAULT_FAMILY: &str = "JetBrainsMono Nerd Font";

/// Size `HyDE` falls back to when nothing names one, in pixels.
const DEFAULT_SIZE: f32 = 10.0;

/// Key the family is stated under in `config.toml`.
const CONFIG_FAMILY_KEY: &str = "WAYBAR_FONT";

/// Key the size is stated under in `config.toml`.
const CONFIG_SIZE_KEY: &str = "WAYBAR_SCALE";

/// Name the family is stated under in `hypr.theme` and `staterc`.
const BAR_FONT: &str = "BAR_FONT";

/// Name the size is stated under in `hypr.theme` and `staterc`.
const BAR_FONT_SIZE: &str = "BAR_FONT_SIZE";

/// Key `staterc` records the active theme under.
const THEME_KEY: &str = "HYDE_THEME";

/// The font a `HyDE` install means the bar to render with.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct BarFont {
    /// Family name, as a font backend would look it up.
    pub family:  String,
    /// Size in pixels.
    pub size_px: f32
}

/// Resolves the bar font of a `HyDE` install.
///
/// Always answers, because every link of the chain ends in a `HyDE` default; a
/// caller that must not impose those defaults on a machine without `HyDE`
/// should check [`HydeDirs::is_installed`] first.
#[must_use]
pub(super) fn read(dirs: &HydeDirs) -> BarFont {
    let sources = Sources::read(dirs);

    BarFont {
        family:  sources
            .family()
            .unwrap_or_else(|| DEFAULT_FAMILY.to_owned()),
        size_px: sources.size().unwrap_or(DEFAULT_SIZE)
    }
}

/// The four files the font is resolved out of, read once.
///
/// Held together because the family and the size walk the same files in a
/// different order, and reading each file twice would let the two disagree if a
/// theme switch landed in between.
struct Sources {
    config:     String,
    hypr_theme: String,
    staterc:    String,
    env_theme:  String
}

impl Sources {
    /// Reads every file the font may be stated in.
    ///
    /// A missing file reads as empty rather than as an error: an install that
    /// has never been themed is missing most of them, and the chain is designed
    /// to fall through. One that is there and refuses to be read is named in
    /// the journal on the way past.
    fn read(dirs: &HydeDirs) -> Self {
        let staterc = hyde_files::text_or_empty(&dirs.staterc());
        let hypr_theme = shell_vars::value_of(&staterc, THEME_KEY)
            .map(|theme| hyde_files::text_or_empty(&dirs.hypr_theme(&theme)))
            .unwrap_or_default();
        let config = hyde_files::text(&dirs.hyde_config_export())
            .or_else(|| hyde_files::text(&dirs.hyde_config()))
            .unwrap_or_default();

        Self {
            config,
            hypr_theme,
            staterc,
            env_theme: hyde_files::text_or_empty(&dirs.env_theme())
        }
    }

    /// Walks the family chain.
    fn family(&self) -> Option<String> {
        shell_vars::value_of(&self.config, CONFIG_FAMILY_KEY)
            .or_else(|| hypr_vars::value_of(&self.hypr_theme, BAR_FONT))
            .or_else(|| shell_vars::value_of(&self.staterc, BAR_FONT))
            .or_else(|| shell_vars::value_of(&self.env_theme, BAR_FONT))
    }

    /// Walks the size chain.
    ///
    /// A size that could not be rendered — zero, negative or not a number —
    /// stops the walk and lands on the default rather than being handed to the
    /// renderer, because a bar with an unreadable font is worse than one with
    /// the wrong size.
    fn size(&self) -> Option<f32> {
        shell_vars::number(&self.config, CONFIG_SIZE_KEY)
            .or_else(|| shell_vars::number(&self.staterc, BAR_FONT_SIZE))
            .or_else(|| {
                hypr_vars::value_of(&self.hypr_theme, BAR_FONT_SIZE)
                    .and_then(|value| value.parse().ok())
            })
            .or_else(|| shell_vars::number(&self.env_theme, BAR_FONT_SIZE))
            .filter(|size: &f32| size.is_finite() && *size > 0.0)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::{fs, path::PathBuf};

    use super::*;

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

        fn write(self, path: PathBuf, contents: &str) -> Self {
            fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
            fs::write(path, contents).expect("write fixture");
            self
        }

        fn staterc(self, contents: &str) -> Self {
            let path = self.dirs.staterc();
            self.write(path, contents)
        }

        fn hypr_theme(self, theme: &str, contents: &str) -> Self {
            let path = self.dirs.hypr_theme(theme);
            self.write(path, contents)
        }

        fn hyde_config(self, contents: &str) -> Self {
            let path = self.dirs.hyde_config();
            self.write(path, contents)
        }

        fn config_export(self, contents: &str) -> Self {
            let path = self.dirs.hyde_config_export();
            self.write(path, contents)
        }

        fn env_theme(self, contents: &str) -> Self {
            let path = self.dirs.env_theme();
            self.write(path, contents)
        }
    }

    #[test]
    fn an_install_that_names_nothing_falls_back_to_the_hyde_defaults() {
        let font = read(&Install::new().dirs);

        assert_eq!(font.family, "JetBrainsMono Nerd Font");
        assert_eq!(font.size_px, 10.0);
    }

    #[test]
    fn the_shipped_fallbacks_are_used_when_nothing_above_them_names_a_family() {
        let install = Install::new().env_theme("BAR_FONT=\"Iosevka Nerd Font\"\n");

        assert_eq!(read(&install.dirs).family, "Iosevka Nerd Font");
    }

    #[test]
    fn the_state_file_outranks_the_shipped_fallbacks() {
        let install = Install::new()
            .env_theme("BAR_FONT=\"Iosevka Nerd Font\"\n")
            .staterc("BAR_FONT=\"Fira Sans\"\n");

        assert_eq!(read(&install.dirs).family, "Fira Sans");
    }

    #[test]
    fn the_theme_fragment_outranks_the_state_file_for_the_family() {
        let install = Install::new()
            .staterc("HYDE_THEME=\"Fixture\"\nBAR_FONT=\"Fira Sans\"\n")
            .hypr_theme("Fixture", "$BAR_FONT = JetBrainsMono Nerd Font\n");

        assert_eq!(read(&install.dirs).family, "JetBrainsMono Nerd Font");
    }

    #[test]
    fn the_hyde_settings_file_outranks_every_theme() {
        let install = Install::new()
            .staterc("HYDE_THEME=\"Fixture\"\nBAR_FONT=\"Fira Sans\"\n")
            .hypr_theme("Fixture", "$BAR_FONT = JetBrainsMono Nerd Font\n")
            .hyde_config("WAYBAR_FONT=\"Cantarell\"\n");

        assert_eq!(read(&install.dirs).family, "Cantarell");
    }

    #[test]
    fn the_flat_export_outranks_the_settings_file_it_was_generated_from() {
        let install = Install::new()
            .hyde_config("WAYBAR_FONT=\"Cantarell\"\n")
            .config_export("export WAYBAR_FONT=\"Mononoki Nerd Font\"\n");

        assert_eq!(read(&install.dirs).family, "Mononoki Nerd Font");
    }

    #[test]
    fn a_sectioned_settings_file_yields_to_the_export_that_flattens_it() {
        let install = Install::new()
            .hyde_config("[waybar]\nfont = \"Cantarell\"\n")
            .config_export("export WAYBAR_FONT=\"Cantarell\"\nexport WAYBAR_SCALE=\"12\"\n");

        let font = read(&install.dirs);
        assert_eq!(font.family, "Cantarell");
        assert_eq!(font.size_px, 12.0);
    }

    #[test]
    fn an_empty_value_falls_through_instead_of_blanking_the_font() {
        let install = Install::new()
            .staterc("HYDE_THEME=\"Fixture\"\nBAR_FONT=\"Fira Sans\"\n")
            .hyde_config("WAYBAR_FONT=\"\"\n");

        assert_eq!(read(&install.dirs).family, "Fira Sans");
    }

    #[test]
    fn the_state_file_outranks_the_theme_fragment_for_the_size() {
        let install = Install::new()
            .staterc("HYDE_THEME=\"Fixture\"\nBAR_FONT_SIZE=\"12\"\n")
            .hypr_theme("Fixture", "$BAR_FONT_SIZE = 9\n");

        assert_eq!(read(&install.dirs).size_px, 12.0);
    }

    #[test]
    fn the_theme_fragment_names_the_size_when_the_state_file_does_not() {
        let install = Install::new()
            .staterc("HYDE_THEME=\"Fixture\"\n")
            .hypr_theme("Fixture", "$BAR_FONT_SIZE = 9\n");

        assert_eq!(read(&install.dirs).size_px, 9.0);
    }

    #[test]
    fn the_hyde_settings_file_outranks_every_size_below_it() {
        let install = Install::new()
            .staterc("HYDE_THEME=\"Fixture\"\nBAR_FONT_SIZE=\"12\"\n")
            .hypr_theme("Fixture", "$BAR_FONT_SIZE = 9\n")
            .hyde_config("WAYBAR_SCALE=\"14\"\n");

        assert_eq!(read(&install.dirs).size_px, 14.0);
    }

    #[test]
    fn a_size_that_could_not_be_rendered_falls_back_to_the_default() {
        let install = Install::new().staterc("BAR_FONT_SIZE=\"0\"\n");

        assert_eq!(read(&install.dirs).size_px, 10.0);
    }

    #[test]
    fn a_theme_the_state_file_does_not_name_contributes_nothing() {
        let install = Install::new().hypr_theme("Fixture", "$BAR_FONT = Should Not Be Read\n");

        assert_eq!(read(&install.dirs).family, "JetBrainsMono Nerd Font");
    }
}
