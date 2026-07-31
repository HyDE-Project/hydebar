//! Where the HyDE Project keeps the files the bar reads.
//!
//! HyDE spreads itself over all four XDG roots: the installation and the themes
//! are configuration, the active selection is session state, the generated
//! palettes are cache and the shipped fallbacks are shared data. Reading any
//! one value therefore needs more than one root, and every reader used to
//! resolve them again from the environment — which made it possible for two
//! readers to disagree about where HyDE lives.
//!
//! This module resolves them once and names every file the bar reads, so the
//! layout of a HyDE install is stated in one place instead of being spelled out
//! again at each call site.

use std::path::{Path, PathBuf};

/// The four roots HyDE spreads its files across.
///
/// Held as one value rather than passed around separately so a caller cannot
/// pair the configuration of one install with the state of another, which is
/// exactly what happens when tests point at a temporary directory but a reader
/// still reaches for `$HOME`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HydeDirs {
    /// XDG configuration root, typically `~/.config`.
    pub config: PathBuf,
    /// XDG state root, typically `~/.local/state`.
    pub state:  PathBuf,
    /// XDG cache root, typically `~/.cache`.
    pub cache:  PathBuf,
    /// XDG data root, typically `~/.local/share`.
    pub data:   PathBuf
}

impl HydeDirs {
    /// Builds the roots explicitly, which is what tests and callers with their
    /// own layout need.
    #[must_use]
    pub fn new(config: PathBuf, state: PathBuf, cache: PathBuf, data: PathBuf) -> Self {
        Self {
            config,
            state,
            cache,
            data
        }
    }

    /// Resolves the roots of the running session.
    ///
    /// Returns [`None`] when the environment names neither the XDG variables
    /// nor a home directory: there is then no HyDE install to read and
    /// guessing a path would only produce misleading failures.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        Some(Self::new(
            xdg_dir("XDG_CONFIG_HOME", ".config")?,
            xdg_dir("XDG_STATE_HOME", ".local/state")?,
            xdg_dir("XDG_CACHE_HOME", ".cache")?,
            xdg_dir("XDG_DATA_HOME", ".local/share")?
        ))
    }

    /// Whether HyDE is installed at all.
    ///
    /// The bar has to tell "HyDE is here but has not written this value" from
    /// "there is no HyDE": in the first case the documented HyDE default is the
    /// right answer, in the second imposing HyDE's fonts and colours on a
    /// desktop that never asked for them would be wrong.
    #[must_use]
    pub fn is_installed(&self) -> bool {
        self.hyde_config_dir().is_dir() || self.hyde_state_dir().is_dir()
    }

    /// Directory the HyDE installation lives in, `~/.config/hyde`.
    #[must_use]
    pub fn hyde_config_dir(&self) -> PathBuf {
        self.config.join("hyde")
    }

    /// Directory HyDE records the live session in, `~/.local/state/hyde`.
    #[must_use]
    pub fn hyde_state_dir(&self) -> PathBuf {
        self.state.join("hyde")
    }

    /// Directory HyDE generates into, `~/.cache/hyde`.
    ///
    /// Named separately from the files inside it because the palette is a
    /// symlink that gets replaced, so a watcher has to follow this directory
    /// rather than any file within it.
    #[must_use]
    pub fn hyde_cache_dir(&self) -> PathBuf {
        self.cache.join("hyde")
    }

    /// The HyDE settings file, `~/.config/hyde/config.toml`.
    ///
    /// First link of every value chain: what the user wrote here outranks
    /// anything a theme or the session state says.
    #[must_use]
    pub fn hyde_config(&self) -> PathBuf {
        self.hyde_config_dir().join("config.toml")
    }

    /// The flat settings export, `~/.local/state/hyde/config`.
    ///
    /// The `hyde-config` daemon parses `config.toml` into `export KEY=value`
    /// lines here, and HyDE's own tools read the settings from this export
    /// rather than from the TOML. The bar does the same: the TOML grew
    /// sections a shell-assignment parser cannot follow, while the export
    /// stays flat by construction.
    #[must_use]
    pub fn hyde_config_export(&self) -> PathBuf {
        self.hyde_state_dir().join("config")
    }

    /// The session state file, `~/.local/state/hyde/staterc`.
    ///
    /// Carries the active theme, `enableWallDcol` and the bar font overrides,
    /// and is the first file a theme switch writes.
    #[must_use]
    pub fn staterc(&self) -> PathBuf {
        self.hyde_state_dir().join("staterc")
    }

    /// Directory the installed themes live in, `~/.config/hyde/themes`.
    #[must_use]
    pub fn themes_dir(&self) -> PathBuf {
        self.hyde_config_dir().join("themes")
    }

    /// Directory of one named theme.
    #[must_use]
    pub fn theme_dir(&self, theme: &str) -> PathBuf {
        self.themes_dir().join(theme)
    }

    /// Palette generated from the current wallpaper, `~/.cache/hyde/wall.dcol`.
    ///
    /// A symlink into `~/.cache/hyde/dcols`, re-pointed on every wallpaper
    /// change; it is read through the link so the bar always sees the palette
    /// the rest of the desktop was coloured from.
    #[must_use]
    pub fn wall_dcol(&self) -> PathBuf {
        self.hyde_cache_dir().join("wall.dcol")
    }

    /// Palette a theme ships to override the wallpaper colours,
    /// `~/.config/hyde/themes/<theme>/theme.dcol`.
    #[must_use]
    pub fn theme_dcol(&self, theme: &str) -> PathBuf {
        self.theme_dir(theme).join("theme.dcol")
    }

    /// Hyprland fragment a theme ships,
    /// `~/.config/hyde/themes/<theme>/hypr.theme`.
    ///
    /// Holds the theme's `$BAR_FONT` and `$COLOR_SCHEME`, both of which decide
    /// how the bar paints itself.
    #[must_use]
    pub fn hypr_theme(&self, theme: &str) -> PathBuf {
        self.theme_dir(theme).join("hypr.theme")
    }

    /// User override of the Hyprland variables,
    /// `~/.local/state/hyde/hyprland.conf`.
    ///
    /// HyDE reads this after the theme's own fragment, so a variable set here
    /// wins over the theme.
    #[must_use]
    pub fn hyprland_override(&self) -> PathBuf {
        self.hyde_state_dir().join("hyprland.conf")
    }

    /// Fallback values HyDE ships, `~/.local/share/hyde/env-theme`.
    ///
    /// Last link of the font chain before the built-in default, and the only
    /// place a stock install states `BAR_FONT` at all.
    #[must_use]
    pub fn env_theme(&self) -> PathBuf {
        self.data.join("hyde").join("env-theme")
    }

    /// Bar layouts HyDE ships, `~/.local/share/waybar/layouts`.
    ///
    /// The directory carries the name of the bar HyDE shipped with before this
    /// one; the path is read as found because that is where the layouts are.
    #[must_use]
    pub fn bar_layouts_dir(&self) -> PathBuf {
        self.data.join("waybar").join("layouts")
    }
}

/// Resolves an XDG root, falling back to a path relative to the home directory.
fn xdg_dir(key: &str, fallback: &str) -> Option<PathBuf> {
    if let Some(dir) = non_empty_var(key) {
        return Some(PathBuf::from(dir));
    }

    non_empty_var("HOME").map(|home| Path::new(&home).join(fallback))
}

/// Returns an environment variable only when it holds a non empty value.
fn non_empty_var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs() -> HydeDirs {
        HydeDirs::new(
            PathBuf::from("/root/config"),
            PathBuf::from("/root/state"),
            PathBuf::from("/root/cache"),
            PathBuf::from("/root/data")
        )
    }

    #[test]
    fn every_file_is_placed_under_the_root_that_owns_it() {
        let dirs = dirs();

        assert_eq!(
            dirs.hyde_config(),
            Path::new("/root/config/hyde/config.toml")
        );
        assert_eq!(dirs.staterc(), Path::new("/root/state/hyde/staterc"));
        assert_eq!(dirs.wall_dcol(), Path::new("/root/cache/hyde/wall.dcol"));
        assert_eq!(dirs.env_theme(), Path::new("/root/data/hyde/env-theme"));
        assert_eq!(
            dirs.hyprland_override(),
            Path::new("/root/state/hyde/hyprland.conf")
        );
    }

    #[test]
    fn a_theme_keeps_its_files_together() {
        let dirs = dirs();

        assert_eq!(
            dirs.theme_dir("Gruvbox Retro"),
            Path::new("/root/config/hyde/themes/Gruvbox Retro")
        );
        assert_eq!(
            dirs.theme_dcol("Gruvbox Retro"),
            Path::new("/root/config/hyde/themes/Gruvbox Retro/theme.dcol")
        );
        assert_eq!(
            dirs.hypr_theme("Gruvbox Retro"),
            Path::new("/root/config/hyde/themes/Gruvbox Retro/hypr.theme")
        );
    }

    #[test]
    fn roots_without_a_hyde_directory_read_as_not_installed() {
        assert!(!dirs().is_installed());
    }

    #[test]
    fn a_configuration_directory_alone_is_enough_to_read_as_installed() {
        let root = tempfile::TempDir::new().expect("tempdir");
        std::fs::create_dir_all(root.path().join("config").join("hyde"))
            .expect("create hyde config directory");

        let dirs = HydeDirs::new(
            root.path().join("config"),
            root.path().join("state"),
            root.path().join("cache"),
            root.path().join("data")
        );

        assert!(dirs.is_installed());
    }
}
