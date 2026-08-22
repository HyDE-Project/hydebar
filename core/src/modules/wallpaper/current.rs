//! The wallpaper standing on the screen right now, as a picture to draw.
//!
//! `HyDE` keeps the wallpaper in force behind a handful of links in its cache:
//! the picture itself, a square crop, a blurred copy and a thumbnail. The
//! thumbnail is what a preview wants — it is already small, already decoded
//! from whatever the original was, and it moves the instant the wallpaper does.
//!
//! Read off the drawing thread and only when something says the wallpaper
//! moved: decoding even a thumbnail is a hundredth of a second the bar has no
//! business spending on a frame.

use std::path::{Path, PathBuf};

use iced::widget::image::Handle;

/// Links `HyDE` keeps the wallpaper in force behind, best first.
///
/// The thumbnail is the one worth decoding. The square crop is the fallback
/// for a cache that has not been filled yet, and the wallpaper itself is the
/// last resort: it is the full picture, so it costs the most to decode and is
/// only reached for on a desktop that keeps no thumbnails at all.
const LINKS: [&str; 3] = ["wall.thmb", "wall.sqre", "wall.set"];

/// Longest side the preview is scaled to, in pixels.
///
/// Wide enough to stay sharp on a column of a four megapixel screen and small
/// enough that the decode is not felt.
const PREVIEW_SIDE: u32 = 512;

/// The wallpaper in force, decoded ready to draw.
///
/// The path is handed back beside the picture so a caller can tell one
/// wallpaper from the next without decoding a second time. [`None`] on a
/// desktop that keeps no such cache, which is every desktop but this one.
#[must_use]
pub fn current() -> Option<(PathBuf, Handle)> {
    let cache = cache_dir()?;
    let path = LINKS
        .iter()
        .map(|link| cache.join(link))
        .find(|path| path.exists())?;
    let read = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());

    Some((read.clone(), decode(&read)?))
}

/// Where the desktop keeps the links, on a session that keeps them.
fn cache_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;

    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map_or_else(|| Path::new(&home).join(".cache"), PathBuf::from)
        .join("hyde");

    cache.is_dir().then_some(cache)
}

/// Decodes one picture down to the side a preview needs.
fn decode(path: &Path) -> Option<Handle> {
    let decoded = ::image::load_from_memory(&std::fs::read(path).ok()?)
        .ok()?
        .thumbnail(PREVIEW_SIDE, PREVIEW_SIDE)
        .into_rgba8();
    let (width, height) = decoded.dimensions();

    Some(Handle::from_rgba(width, height, decoded.into_raw()))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_thumbnail_is_preferred_over_the_whole_picture() {
        assert_eq!(LINKS[0], "wall.thmb");
        assert_eq!(LINKS[LINKS.len() - 1], "wall.set");
    }

    #[test]
    fn a_session_without_the_cache_answers_with_nothing() {
        // the reader is only ever right on a desktop that keeps the links, and
        // it must say so rather than hand back a blank picture
        assert!(decode(Path::new("/nonexistent/wallpaper.png")).is_none());
    }
}
