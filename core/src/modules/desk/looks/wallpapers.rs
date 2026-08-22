//! The wallpapers of the theme in force, as the crops `HyDE` keeps of them.

use std::path::{Path, PathBuf};

use hydebar_proto::hyde_dirs::HydeDirs;

use super::reel::{Reel, reel};
use crate::modules::wallpaper::listing;

/// The link `HyDE` points the wallpaper on screen at.
const IN_FORCE: &str = "wall.set";

/// The reel of the theme's wallpapers, centred on the one on screen.
///
/// Listed by the desktop itself rather than by walking a directory, so the
/// reel holds the very pictures the desktop's own picker offers and holds
/// them in the same order.
pub(super) fn wallpapers(dirs: &HydeDirs) -> Reel {
    let listed: Vec<(String, PathBuf)> = listing::listed()
        .into_iter()
        .map(|entry| (named(&entry.basename), PathBuf::from(entry.path)))
        .collect();
    let at = on_screen(dirs)
        .and_then(|picture| listed.iter().position(|(_, path)| *path == picture))
        .unwrap_or_default();

    reel(&listed, at)
}

/// What a wallpaper is called, which is its file name without the extension.
fn named(basename: &str) -> String {
    Path::new(basename).file_stem().map_or_else(
        || basename.to_owned(),
        |stem| stem.to_string_lossy().into_owned()
    )
}

/// The wallpaper standing on the screen right now, as a path on disk.
fn on_screen(dirs: &HydeDirs) -> Option<PathBuf> {
    std::fs::canonicalize(dirs.hyde_cache_dir().join(IN_FORCE)).ok()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_wallpaper_is_called_by_its_own_name_without_the_extension() {
        assert_eq!(named("cat_lofi_cafe.jpg"), "cat_lofi_cafe");
        assert_eq!(named("no-extension"), "no-extension");
    }
}
