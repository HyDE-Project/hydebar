//! The pictures `HyDE` already keeps of a theme and of a wallpaper.
//!
//! A theme's look is a picture, not a sentence, and the picture exists on this
//! machine before the bar asks for it: `HyDE` caches a square crop and a large
//! thumbnail of every wallpaper it has ever set, keyed by the digest of the
//! file. Its own theme picker draws exactly these, so a bar that finds them the
//! same way shows the user the very image they picked the theme by.
//!
//! Only the paths are answered here. Decoding belongs to whoever is going to
//! draw them, and this crate draws nothing.

use std::path::{Path, PathBuf};

use super::swatch::wallpaper_digest;
use crate::hyde_dirs::HydeDirs;

/// Directory `HyDE` keeps its cached thumbnails in.
const THUMBS: &str = "thumbs";

/// Cached crops of one picture, cheapest first.
///
/// The square crop is what a preview wants: it is small, already decoded from
/// whatever the original was, and it is the very crop `HyDE`'s own picker
/// draws. The large thumbnail answers for a cache filled before the square
/// crops existed.
const CROPS: [&str; 2] = ["sqre", "thmb"];

/// The link a theme points its current wallpaper at.
const THEME_WALLPAPER: &str = "wall.set";

/// The picture that stands for `theme`, ready to be decoded.
///
/// The theme's own wallpaper, as its cached crop where there is one and as the
/// picture itself where there is not. [`None`] when the theme is not installed
/// or points at a wallpaper that is gone.
#[must_use]
pub fn theme_preview(dirs: &HydeDirs, theme: &str) -> Option<PathBuf> {
    let wallpaper = std::fs::canonicalize(dirs.theme_dir(theme).join(THEME_WALLPAPER)).ok()?;

    picture_preview(dirs, &wallpaper)
}

/// The cheapest drawable form of the picture at `image`.
///
/// The cached crop when `HyDE` has one, the picture itself otherwise, and
/// [`None`] when neither is on disk.
#[must_use]
pub fn picture_preview(dirs: &HydeDirs, image: &Path) -> Option<PathBuf> {
    let cached = wallpaper_digest(image)
        .map(|digest| dirs.hyde_cache_dir().join(THUMBS).join(digest))
        .into_iter()
        .flat_map(|base| CROPS.map(|crop| base.with_extension(crop)))
        .find(|path| path.is_file());

    cached.or_else(|| image.is_file().then(|| image.to_owned()))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

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

    fn wallpaper(dirs: &HydeDirs, theme: &str) -> PathBuf {
        let directory = dirs.theme_dir(theme).join("wallpapers");
        fs::create_dir_all(&directory).expect("theme directory");

        let picture = directory.join("mountains.jpg");
        fs::write(&picture, b"a picture, for the purposes of a digest").expect("wallpaper");
        fs::write(dirs.theme_dir(theme).join(THEME_WALLPAPER), b"").expect("link stand-in");

        picture
    }

    /// The crop is the point: a preview that handed back the wallpaper itself
    /// would have the bar decode a four megapixel picture per theme.
    #[test]
    fn the_cached_crop_is_preferred_over_the_picture_itself() {
        let (_root, dirs) = install();
        let picture = wallpaper(&dirs, "Nordic Blue");
        let digest = wallpaper_digest(&picture).expect("digest");
        let thumbs = dirs.hyde_cache_dir().join(THUMBS);

        fs::create_dir_all(&thumbs).expect("thumbs");
        fs::write(thumbs.join(format!("{digest}.sqre")), b"a crop").expect("crop");

        assert_eq!(
            picture_preview(&dirs, &picture),
            Some(thumbs.join(format!("{digest}.sqre")))
        );
    }

    #[test]
    fn a_picture_with_no_crop_is_drawn_from_itself() {
        let (_root, dirs) = install();
        let picture = wallpaper(&dirs, "Nordic Blue");

        assert_eq!(picture_preview(&dirs, &picture), Some(picture));
    }

    #[test]
    fn a_picture_that_is_gone_has_no_preview() {
        let (_root, dirs) = install();

        assert_eq!(
            picture_preview(&dirs, Path::new("/nonexistent/wall.png")),
            None
        );
    }

    #[test]
    fn a_theme_that_is_not_installed_has_no_preview() {
        let (_root, dirs) = install();

        assert_eq!(theme_preview(&dirs, "Never Installed"), None);
    }
}
