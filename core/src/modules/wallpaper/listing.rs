//! Reading the theme's wallpapers from the desktop and decoding thumbnails.

use serde::Deserialize;

use super::WallpaperEntry;

/// One wallpaper as the desktop lists it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct ListedWallpaper {
    /// Full path of the picture, what a set command takes.
    path:     String,
    /// File name, the tile's caption.
    basename: String,
    /// Square thumbnail `HyDE` keeps in its cache.
    sqre:     String
}

/// Side the decoded thumbnails are scaled to, in pixels.
const THUMB_SIDE: u32 = 256;

/// Reads the wallpapers of the theme in force from the desktop.
///
/// A failure answers with an empty list and the picker says so; the desktop
/// not being `HyDE` is not an error the bar can fix.
pub(super) fn list_wallpapers(
    known: &std::collections::HashMap<String, iced::widget::image::Handle>
) -> Vec<WallpaperEntry> {
    let listing = std::process::Command::new("timeout")
        .args(["10", "hydectl", "wallpaper", "list"])
        .output();

    let Ok(output) = listing else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let listed: Vec<ListedWallpaper> = serde_json::from_slice(&output.stdout).unwrap_or_default();

    listed
        .into_iter()
        .filter_map(|entry| {
            if let Some(thumbnail) = known.get(&entry.path) {
                return Some(WallpaperEntry {
                    path:      entry.path,
                    thumbnail: thumbnail.clone()
                });
            }

            let decoded = std::fs::read(&entry.sqre)
                .ok()
                .and_then(|bytes| ::image::load_from_memory(&bytes).ok())
                .or_else(|| {
                    std::fs::read(&entry.path)
                        .ok()
                        .and_then(|bytes| ::image::load_from_memory(&bytes).ok())
                })?
                .thumbnail(THUMB_SIDE, THUMB_SIDE)
                .into_rgba8();
            let (width, height) = decoded.dimensions();

            Some(WallpaperEntry {
                path:      entry.path,
                thumbnail: iced::widget::image::Handle::from_rgba(
                    width,
                    height,
                    decoded.into_raw()
                )
            })
        })
        .collect()
}
