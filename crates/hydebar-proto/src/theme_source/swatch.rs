//! The colours a theme would paint the desktop in, read for previewing it.
//!
//! A list of theme names answers "which themes are there", not "what do they
//! look like". The look is on disk already: a theme either pins its palette in
//! its own palette file, or its current wallpaper has one extracted and cached
//! by HyDE the last time that wallpaper was applied. Reading those gives every
//! entry of the theme list the colours it would actually bring, without
//! switching to it.

use std::fs;

use super::dcol::DcolPalette;
use crate::{hyde_dirs::HydeDirs, theme_source::Rgba};

/// The colours a theme announces itself with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeSwatch {
    /// Surface the theme paints things on.
    pub background: Rgba,
    /// Text the theme sets on that surface.
    pub text:       Rgba,
    /// Accent the theme highlights with.
    pub accent:     Rgba
}

/// Reads the swatch of `theme`, if anything on disk answers for its colours.
///
/// The theme's own pinned palette wins; the palette extracted from its
/// current wallpaper answers otherwise. A theme with neither — never applied
/// on this machine and shipping no palette — has no swatch, and the caller
/// paints its entry the way it paints everything.
#[must_use]
pub fn theme_swatch(dirs: &HydeDirs, theme: &str) -> Option<ThemeSwatch> {
    let palette = pinned_palette(dirs, theme).or_else(|| wallpaper_palette(dirs, theme))?;

    Some(ThemeSwatch {
        background: palette.primary[0],
        text:       palette.text[0],
        accent:     palette.primary[3]
    })
}

/// The palette the theme ships under its own name.
fn pinned_palette(dirs: &HydeDirs, theme: &str) -> Option<DcolPalette> {
    DcolPalette::parse(&fs::read_to_string(dirs.theme_dcol(theme)).ok()?)
}

/// The palette HyDE extracted from the theme's current wallpaper.
///
/// The cache is keyed by the digest of the image file, exactly as the scripts
/// key it, so the bar finds the palette wherever the wallpaper file lives.
fn wallpaper_palette(dirs: &HydeDirs, theme: &str) -> Option<DcolPalette> {
    let image = fs::canonicalize(dirs.theme_dir(theme).join("wall.set")).ok()?;
    let bytes = fs::read(image).ok()?;
    let digest = sha1_smol::Sha1::from(&bytes).digest().to_string();
    let cached = dirs
        .hyde_cache_dir()
        .join("dcols")
        .join(format!("{digest}.dcol"));

    DcolPalette::parse(&fs::read_to_string(cached).ok()?)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{super::dcol::fixtures::WALL_DCOL, *};

    fn install() -> (TempDir, HydeDirs) {
        let root = TempDir::new().expect("tempdir");
        let dirs = HydeDirs::new(
            root.path().join("config"),
            root.path().join("state"),
            root.path().join("cache"),
            root.path().join("data")
        );

        (root, dirs)
    }

    #[test]
    fn a_pinned_palette_answers_for_the_theme() {
        let (_root, dirs) = install();
        let theme_dir = dirs.theme_dir("Nord");
        fs::create_dir_all(&theme_dir).expect("theme dir");
        fs::write(dirs.theme_dcol("Nord"), WALL_DCOL).expect("palette");

        let swatch = theme_swatch(&dirs, "Nord").expect("swatch");

        assert_eq!(swatch.background, Rgba::rgb(0x48, 0x38, 0x28));
    }

    #[test]
    fn a_cached_wallpaper_palette_answers_when_nothing_is_pinned() {
        let (root, dirs) = install();
        let theme_dir = dirs.theme_dir("Nord");
        fs::create_dir_all(&theme_dir).expect("theme dir");

        let image = root.path().join("wall.png");
        fs::write(&image, b"not really a picture").expect("image");
        std::os::unix::fs::symlink(&image, theme_dir.join("wall.set")).expect("link");

        let digest = sha1_smol::Sha1::from(b"not really a picture")
            .digest()
            .to_string();
        let dcols = dirs.hyde_cache_dir().join("dcols");
        fs::create_dir_all(&dcols).expect("dcols dir");
        fs::write(dcols.join(format!("{digest}.dcol")), WALL_DCOL).expect("cache");

        assert!(theme_swatch(&dirs, "Nord").is_some());
    }

    #[test]
    fn a_theme_with_no_palette_anywhere_has_no_swatch() {
        let (_root, dirs) = install();

        assert_eq!(theme_swatch(&dirs, "Ghost"), None);
    }
}
