//! The theme snapshot and the entry points that read it from disk.

use std::{
    fs,
    path::{Path, PathBuf}
};

use super::{
    color::Rgba,
    css::strip_comments,
    extract::{apply_colors, apply_global, parse_pill_radius}
};

/// Snapshot of the HyDE theme as read from the Waybar stylesheets.
///
/// Every field is independent: a missing or malformed source leaves only the
/// affected fields as [`None`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HydeTheme {
    /// Background of the bar surface (`bar-bg`).
    pub bar_background:    Option<Rgba>,
    /// Background of an individual module (`main-bg`).
    pub module_background: Option<Rgba>,
    /// Default foreground color (`main-fg`).
    pub text:              Option<Rgba>,
    /// Background of an active module (`wb-act-bg`).
    pub active_background: Option<Rgba>,
    /// Foreground of an active module (`wb-act-fg`).
    pub active_text:       Option<Rgba>,
    /// Background of a hovered module (`wb-hvr-bg`).
    pub hover_background:  Option<Rgba>,
    /// Foreground of a hovered module (`wb-hvr-fg`).
    pub hover_text:        Option<Rgba>,
    /// Font family declared for every widget.
    pub font_family:       Option<String>,
    /// Font size in CSS pixels.
    pub font_size_px:      Option<f32>,
    /// Corner radius of a full pill, in CSS pixels.
    pub radius_px:         Option<f32>
}

/// Reads the HyDE theme from the Waybar stylesheets under `config_dir`.
///
/// `config_dir` is the XDG configuration root (typically `~/.config`); the
/// Waybar files are looked up under `<config_dir>/waybar`.
///
/// This function never fails. Unreadable files, unknown keys and unparsable
/// values are skipped.
#[must_use]
pub fn load_from(config_dir: &Path) -> HydeTheme {
    let waybar = config_dir.join("waybar");

    let mut theme = HydeTheme::default();

    if let Some(source) = read_stylesheet(&waybar.join("theme.css")) {
        apply_colors(&mut theme, &source);
    }

    if let Some(source) = read_stylesheet(&waybar.join("includes").join("global.css")) {
        apply_global(&mut theme, &source);
    }

    if let Some(source) = read_stylesheet(&waybar.join("includes").join("border-radius.css")) {
        theme.radius_px = parse_pill_radius(&source, theme.font_size_px);
    }

    theme
}

/// Reads the HyDE theme from the user's configuration directory.
///
/// The directory is resolved from `XDG_CONFIG_HOME`, falling back to
/// `$HOME/.config`. When neither variable is set the returned theme is empty.
#[must_use]
pub fn load() -> HydeTheme {
    match config_dir() {
        Some(dir) => load_from(&dir),
        None => HydeTheme::default()
    }
}

/// Resolves the XDG configuration directory.
fn config_dir() -> Option<PathBuf> {
    if let Some(dir) = non_empty_var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(dir));
    }

    non_empty_var("HOME").map(|home| PathBuf::from(home).join(".config"))
}

/// Returns an environment variable only when it holds a non empty value.
fn non_empty_var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

/// Reads a stylesheet and removes its comments, returning [`None`] when the
/// file is missing or unreadable.
fn read_stylesheet(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| strip_comments(&raw))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{
        super::fixtures::{assert_close, write_fixture},
        *
    };

    #[test]
    fn loads_a_complete_theme() {
        let dir = TempDir::new().expect("tempdir");
        write_fixture(dir.path());

        let theme = load_from(dir.path());

        assert_eq!(theme.bar_background, Some(Rgba::rgba(27, 29, 28, 0.01)));
        assert_eq!(theme.module_background, Some(Rgba::rgba(27, 29, 28, 0.8)));
        assert_eq!(theme.text, Some(Rgba::rgb(170, 240, 205)));
        assert_eq!(theme.active_background, Some(Rgba::rgb(195, 172, 118)));
        assert_eq!(theme.active_text, Some(Rgba::rgb(255, 240, 204)));
        assert_eq!(theme.hover_text, Some(Rgba::rgba(240, 204, 170, 0.8)));

        let hover_background = theme.hover_background.expect("hover background");
        assert_eq!(
            (hover_background.r, hover_background.g, hover_background.b),
            (125, 108, 75)
        );
        assert_close(hover_background.a, 102.0 / 255.0);

        assert_eq!(
            theme.font_family.as_deref(),
            Some("JetBrainsMono Nerd Font")
        );
        assert_close(theme.font_size_px.expect("font size"), 10.0);
        assert_close(theme.radius_px.expect("radius"), 4.0);
    }

    #[test]
    fn missing_directory_yields_an_empty_theme() {
        let dir = TempDir::new().expect("tempdir");
        let missing = dir.path().join("does-not-exist");

        assert_eq!(load_from(&missing), HydeTheme::default());
    }

    #[test]
    fn partial_sources_leave_other_fields_unset() {
        let dir = TempDir::new().expect("tempdir");
        let waybar = dir.path().join("waybar");
        fs::create_dir_all(&waybar).expect("create waybar directory");
        fs::write(waybar.join("theme.css"), "@define-color main-fg #ffffff;")
            .expect("write theme.css");

        let theme = load_from(dir.path());

        assert_eq!(theme.text, Some(Rgba::rgb(255, 255, 255)));
        assert_eq!(theme.font_family, None);
        assert_eq!(theme.font_size_px, None);
        assert_eq!(theme.radius_px, None);
        assert_eq!(theme.bar_background, None);
    }
}
