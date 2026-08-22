//! The themes installed on this machine, as the pictures that stand for them.

use std::path::PathBuf;

use hydebar_proto::{hyde_dirs::HydeDirs, hyde_state, theme_source::theme_preview};

use super::reel::{Reel, reel};

/// The reel of installed themes, centred on the one in force.
///
/// The order is `HyDE`'s own — its `.sort` files first, the alphabet after —
/// so walking the reel and pressing the desktop's next-theme key travel the
/// same list in the same direction. A theme whose wallpaper is gone is left
/// out rather than drawn blank: a tile with nothing in it says the theme
/// looks like nothing.
pub(super) fn themes(dirs: &HydeDirs) -> Reel {
    let state = hyde_state::load();
    let listed: Vec<(String, PathBuf)> = state
        .themes
        .iter()
        .filter_map(|name| theme_preview(dirs, name).map(|path| (name.clone(), path)))
        .collect();
    let at = state
        .theme
        .as_deref()
        .and_then(|active| listed.iter().position(|(name, _)| name == active))
        .unwrap_or_default();

    reel(&listed, at)
}
