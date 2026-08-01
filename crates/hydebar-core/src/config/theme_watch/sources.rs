//! The directories the desktop theme is read from, and the names inside them
//! that matter.

use std::{ffi::OsString, path::PathBuf};

use hydebar_proto::hyde_dirs::HydeDirs;

/// Palette symlink under `~/.cache/hyde`, re-pointed on every wallpaper change.
const WALL_DCOL: &str = "wall.dcol";

/// State file under `~/.local/state/hyde` that names the theme in force.
const STATERC: &str = "staterc";

/// Flat settings export under `~/.local/state/hyde`, rewritten by the
/// `hyde-config` daemon whenever the user edits `config.toml`.
const CONFIG_EXPORT: &str = "config";

/// Hyprland variable override under `~/.local/state/hyde`, regenerated from
/// `config.toml` alongside the export; it carries the colour scheme.
const HYPRLAND_OVERRIDE: &str = "hyprland.conf";

/// The settings file itself under `~/.config/hyde`, read directly on installs
/// old enough to keep flat assignments in it.
const HYDE_CONFIG: &str = "config.toml";

/// Shipped fallbacks under `~/.local/share/hyde`, the last link of the font
/// and colour-scheme chains.
const ENV_THEME: &str = "env-theme";

/// Stylesheet under `~/.config/waybar` the bar falls back to.
const THEME_CSS: &str = "theme.css";

/// Stylesheets under `~/.config/waybar/includes` the bar falls back to.
const GLOBAL_CSS: &str = "global.css";

/// Stylesheet under `~/.config/waybar/includes` carrying the fallback radius.
const BORDER_RADIUS_CSS: &str = "border-radius.css";

/// A directory to watch together with the files inside it that matter.
///
/// The watch is placed on the directory and never on a file. `wall.dcol` is a
/// symlink `HyDE` replaces with `ln -fs`, and the stylesheets are written to a
/// temporary file and moved into place; a watch bound to the old inode would
/// keep reporting on a file nothing reads any more and would miss the switch
/// entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeWatchTarget {
    /// Directory the inotify watch is placed on.
    pub directory:  PathBuf,
    /// Names inside `directory` that carry part of the theme.
    pub file_names: Vec<OsString>
}

impl ThemeWatchTarget {
    /// Builds a target from a directory and the names to follow inside it.
    fn new(directory: PathBuf, names: &[&str]) -> Self {
        Self {
            directory,
            file_names: names.iter().map(OsString::from).collect()
        }
    }
}

/// The roots the theme is read from.
///
/// Wraps [`HydeDirs`] rather than restating it so the watcher can never end up
/// following a directory the reader does not read.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThemeRoots {
    /// Where `HyDE` keeps the files the theme is assembled from.
    pub dirs: HydeDirs
}

impl ThemeRoots {
    /// Builds the roots from an explicit `HyDE` layout, which is what tests and
    /// callers with their own layout need.
    #[must_use]
    pub const fn new(dirs: HydeDirs) -> Self {
        Self {
            dirs
        }
    }

    /// Resolves the roots the running session uses.
    ///
    /// Returns [`None`] when the environment names no home directory, in which
    /// case there is no install to follow.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        HydeDirs::from_env().map(Self::new)
    }

    /// Lists the directories to watch and the files that matter inside them.
    ///
    /// The `HyDE` directories come first because they are the source of truth;
    /// the waybar directories are included only because the bar still falls
    /// back to them on an install where `HyDE` answers for nothing, and a
    /// fallback that changed without the bar noticing would be just as
    /// stale as no watch at all.
    #[must_use]
    pub fn targets(&self) -> Vec<ThemeWatchTarget> {
        let waybar = self.dirs.config.join("waybar");

        vec![
            ThemeWatchTarget::new(self.dirs.hyde_cache_dir(), &[WALL_DCOL]),
            ThemeWatchTarget::new(
                self.dirs.hyde_state_dir(),
                &[STATERC, CONFIG_EXPORT, HYPRLAND_OVERRIDE]
            ),
            ThemeWatchTarget::new(self.dirs.hyde_config_dir(), &[HYDE_CONFIG]),
            ThemeWatchTarget::new(self.dirs.data.join("hyde"), &[ENV_THEME]),
            ThemeWatchTarget::new(waybar.clone(), &[THEME_CSS]),
            ThemeWatchTarget::new(waybar.join("includes"), &[GLOBAL_CSS, BORDER_RADIUS_CSS]),
        ]
    }
}

/// Collects every watched file name, regardless of the directory it sits in.
///
/// The watcher matches an event by name alone: an inotify event names the file
/// but the directory it came from would have to be recovered from its watch
/// descriptor. A name colliding across two watched directories only costs one
/// redundant reload, which is cheaper than that bookkeeping.
#[must_use]
pub fn watched_names() -> Vec<OsString> {
    [
        WALL_DCOL,
        STATERC,
        CONFIG_EXPORT,
        HYPRLAND_OVERRIDE,
        HYDE_CONFIG,
        ENV_THEME,
        THEME_CSS,
        GLOBAL_CSS,
        BORDER_RADIUS_CSS
    ]
    .iter()
    .map(OsString::from)
    .collect()
}

/// Lists the directories to watch for the running session.
///
/// Returns nothing when the bar does not follow the desktop theme: a
/// configuration that pins its own colours would only be disturbed by a theme
/// switch, so no watch is placed and no inotify descriptor is spent.
#[must_use]
pub fn watch_targets(follow_hyde: bool) -> Vec<ThemeWatchTarget> {
    if !follow_hyde {
        return Vec::new();
    }

    ThemeRoots::from_env()
        .map(|roots| roots.targets())
        .unwrap_or_default()
}
