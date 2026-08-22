//! How the desktop looks, as the pictures it already has of itself.
//!
//! The canvas has room to show the look rather than name it, and the pictures
//! exist before the bar asks: `HyDE` caches a crop of every wallpaper it sets,
//! and its own theme picker draws exactly those. Read the same way here, the
//! canvas shows the user the very images they chose the look by.
//!
//! Two reels: the themes installed on this machine, and the wallpapers of the
//! theme in force. Each is centred on the one in force, and only what a reel
//! can show is decoded — a machine with a hundred wallpapers decodes seven.
//!
//! Three rooms: [`reel`] is the shape a reel takes and the window it keeps,
//! [`themes`] reads the installed themes and [`wallpapers`] the pictures of
//! the theme in force.

mod reel;
mod themes;
mod wallpapers;

pub use reel::{REACH, Reel, Slide};

/// Everything the canvas shows of how the desktop looks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Looks {
    /// The themes installed on this machine, centred on the one in force.
    pub themes:     Reel,
    /// The wallpapers of the theme in force, centred on the one on screen.
    pub wallpapers: Reel
}

/// Reads both reels off the disk, decoding what they will show.
///
/// Blocking, and meant to be: it reads directories, hashes files and decodes
/// pictures. Called off the drawing thread and only when the desktop says the
/// look moved, which is a handful of times a day.
#[must_use]
pub fn looks() -> Looks {
    let Some(dirs) = hydebar_proto::hyde_dirs::HydeDirs::from_env() else {
        return Looks::default();
    };

    Looks {
        themes:     themes::themes(&dirs),
        wallpapers: wallpapers::wallpapers(&dirs)
    }
}
