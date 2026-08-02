//! Last-resort reader for the stylesheets a bar generator leaves behind.
//!
//! **This is not a source of truth and must never be preferred over the `HyDE`
//! directories.** `~/.config/waybar/theme.css` is rendered from the very
//! palette [`super::dcol`] reads, and `includes/global.css` and
//! `includes/border-radius.css` are written by a generator that only runs while
//! a bar of its own is running. The moment that bar is not running those two
//! files freeze, and a reader that trusted them would keep painting the font
//! and the corner radius of whatever theme was in force when it stopped.
//!
//! The files are therefore read only to fill in what a `HyDE` install did not
//! answer for — an install missing its cache, or a desktop with the stylesheets
//! but no `HyDE` at all. Every value found here is a guess about a theme rather
//! than a reading of it.

use std::{fs, path::Path};

use super::{
    css::strip_comments,
    extract::{apply_colors, apply_global, parse_pill_radius},
    theme::HydeTheme
};

/// Reads whatever the stylesheets under `<config_dir>/waybar` still say.
///
/// Never fails: a missing file, an unknown declaration or a value that cannot
/// be understood simply leaves the affected field unset.
#[must_use]
pub(super) fn read(config_dir: &Path) -> HydeTheme {
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
        super::{
            color::Rgba,
            fixtures::{assert_close, write_fixture}
        },
        *
    };

    #[test]
    fn a_complete_set_of_stylesheets_is_read_whole() {
        let dir = TempDir::new().expect("tempdir");
        write_fixture(dir.path());

        let theme = read(dir.path());

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
    fn a_missing_directory_yields_an_empty_theme() {
        let dir = TempDir::new().expect("tempdir");

        assert_eq!(
            read(&dir.path().join("does-not-exist")),
            HydeTheme::default()
        );
    }

    #[test]
    fn partial_stylesheets_leave_the_other_fields_unset() {
        let dir = TempDir::new().expect("tempdir");
        let waybar = dir.path().join("waybar");
        fs::create_dir_all(&waybar).expect("create waybar directory");
        fs::write(waybar.join("theme.css"), "@define-color main-fg #ffffff;")
            .expect("write theme.css");

        let theme = read(dir.path());

        assert_eq!(theme.text, Some(Rgba::rgb(255, 255, 255)));
        assert_eq!(theme.font_family, None);
        assert_eq!(theme.font_size_px, None);
        assert_eq!(theme.radius_px, None);
        assert_eq!(theme.bar_background, None);
    }
}
