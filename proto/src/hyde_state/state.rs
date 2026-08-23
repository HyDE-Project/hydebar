//! The `HyDE` state snapshot and the entry points that read it from disk.

use std::path::{Path, PathBuf};

use super::themes;
use crate::{hyde_files, shell_vars as staterc};

/// Key the active theme is recorded under.
const THEME_KEY: &str = "HYDE_THEME";

/// Key the wallpaper shader is recorded under.
const SHADER_KEY: &str = "HYPR_SHADER";

/// Key the wallpaper recolouring switch is recorded under.
const WALL_DCOL_KEY: &str = "enableWallDcol";

/// Snapshot of the `HyDE` desktop as it stands right now.
///
/// Every field is independent, so a state file that `HyDE` has not written yet
/// leaves the rest of the snapshot usable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HydeState {
    /// Theme currently in force, as `HyDE` names it.
    ///
    /// This is the name the theme switcher expects back, so it is kept verbatim
    /// rather than normalised.
    pub theme:            Option<String>,
    /// Themes installed on this machine, in the order they are listed.
    pub themes:           Vec<String>,
    /// Whether `HyDE` recolours the desktop from the wallpaper.
    ///
    /// The bar shows this because it explains why the colours moved without
    /// anybody touching a theme. The switch is a mode number, not a flag: `0`
    /// leaves the theme's palette in charge and every other setting — plain
    /// recolouring or one of the forced-mirror modes — takes the colours from
    /// the wallpaper, so the reading here has to match the colour engine's.
    pub wallpaper_colors: bool,
    /// Shader applied to the screen, if any.
    pub shader:           Option<String>
}

impl HydeState {
    /// Builds a snapshot from the contents of the state file and a theme list.
    ///
    /// Kept separate from the file reading so the parsing can be exercised
    /// without a filesystem, and so a caller that already holds the text does
    /// not have to write it back out to be understood.
    #[must_use]
    pub fn parse(staterc_source: &str, themes: Vec<String>) -> Self {
        Self {
            theme: staterc::value_of(staterc_source, THEME_KEY),
            themes,
            wallpaper_colors: staterc::number::<u8>(staterc_source, WALL_DCOL_KEY)
                .unwrap_or_default()
                != 0,
            shader: staterc::value_of(staterc_source, SHADER_KEY)
        }
    }

    /// Whether `theme` is the one currently in force.
    ///
    /// The comparison lives here so a page listing the themes does not have to
    /// unwrap the option and get the empty case subtly wrong.
    #[must_use]
    pub fn is_active(&self, theme: &str) -> bool {
        self.theme.as_deref() == Some(theme)
    }
}

/// Reads the `HyDE` state from `state_dir` and the themes from `config_dir`.
///
/// `state_dir` is the XDG state root (typically `~/.local/state`) and
/// `config_dir` the XDG configuration root (typically `~/.config`); the two are
/// taken separately because `HyDE` splits its live state from its installation.
///
/// This function never fails: a missing file or directory only empties the
/// fields it feeds.
#[must_use]
pub fn load_from(state_dir: &Path, config_dir: &Path) -> HydeState {
    let source = hyde_files::text_or_empty(&state_dir.join("hyde").join("staterc"));
    let installed = themes::installed(&config_dir.join("hyde").join("themes"));

    HydeState::parse(&source, installed)
}

/// Reads the `HyDE` state from the user's own directories.
///
/// The roots are resolved from `XDG_STATE_HOME` and `XDG_CONFIG_HOME`, falling
/// back to the paths under `$HOME` that the specification names. When the
/// environment says nothing the snapshot comes back empty.
#[must_use]
pub fn load() -> HydeState {
    match (
        xdg_dir("XDG_STATE_HOME", ".local/state"),
        xdg_dir("XDG_CONFIG_HOME", ".config")
    ) {
        (Some(state), Some(config)) => load_from(&state, &config),
        _ => HydeState::default()
    }
}

/// Resolves an XDG root from its variable, falling back to `fallback` in
/// `$HOME`.
fn xdg_dir(key: &str, fallback: &str) -> Option<PathBuf> {
    if let Some(dir) = non_empty_var(key) {
        return Some(PathBuf::from(dir));
    }

    non_empty_var("HOME").map(|home| PathBuf::from(home).join(fallback))
}

/// Returns an environment variable only when it holds a non empty value.
fn non_empty_var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    const STATERC: &str = r#"
HYDE_THEME="Gruvbox Retro"
HYPR_SHADER="wallbash"
enableWallDcol="1"
WAYBAR_LAYOUT_NAME=hyprdots/02
"#;

    fn installed_themes() -> Vec<String> {
        vec!["Gruvbox Retro".to_owned(), "Tokyo Night".to_owned()]
    }

    #[test]
    fn a_complete_state_file_fills_every_field() {
        let state = HydeState::parse(STATERC, installed_themes());

        assert_eq!(state.theme.as_deref(), Some("Gruvbox Retro"));
        assert_eq!(state.shader.as_deref(), Some("wallbash"));
        assert!(state.wallpaper_colors);
        assert_eq!(state.themes, installed_themes());
    }

    #[test]
    fn an_empty_state_file_leaves_the_snapshot_empty() {
        let state = HydeState::parse("", Vec::new());

        assert_eq!(state, HydeState::default());
    }

    #[test]
    fn a_partial_state_file_leaves_the_other_fields_unset() {
        let state = HydeState::parse("HYDE_THEME=\"Nordic Blue\"\n", Vec::new());

        assert_eq!(state.theme.as_deref(), Some("Nordic Blue"));
        assert_eq!(state.shader, None);
        assert!(!state.wallpaper_colors);
    }

    #[test]
    fn the_recolouring_switch_follows_the_state_file() {
        assert!(!HydeState::parse("enableWallDcol=\"0\"\n", Vec::new()).wallpaper_colors);
        assert!(HydeState::parse("enableWallDcol=\"1\"\n", Vec::new()).wallpaper_colors);
    }

    /// The forced-mirror settings `2` and `3` still recolour from the
    /// wallpaper; reading them as a boolean flag once reported them as off
    /// while the colour engine painted from the wallpaper regardless.
    #[test]
    fn the_forced_mirror_settings_read_as_wallpaper_recolouring() {
        assert!(HydeState::parse("enableWallDcol=\"2\"\n", Vec::new()).wallpaper_colors);
        assert!(HydeState::parse("enableWallDcol=\"3\"\n", Vec::new()).wallpaper_colors);
    }

    /// A value the scripts would fail to read falls back to the default of
    /// `0`, exactly as `${enableWallDcol:-0}` does.
    #[test]
    fn an_unreadable_recolouring_switch_reads_as_off() {
        assert!(!HydeState::parse("enableWallDcol=\"maybe\"\n", Vec::new()).wallpaper_colors);
    }

    #[test]
    fn only_the_theme_in_force_reads_as_active() {
        let state = HydeState::parse(STATERC, installed_themes());

        assert!(state.is_active("Gruvbox Retro"));
        assert!(!state.is_active("Tokyo Night"));
    }

    #[test]
    fn no_theme_is_active_while_the_state_file_names_none() {
        let state = HydeState::parse("", installed_themes());

        assert!(!state.is_active("Gruvbox Retro"));
        assert!(!state.is_active(""));
    }

    #[test]
    fn missing_directories_yield_an_empty_snapshot() {
        let missing = Path::new("/nonexistent/hydebar/hyde");

        assert_eq!(load_from(missing, missing), HydeState::default());
    }
}
