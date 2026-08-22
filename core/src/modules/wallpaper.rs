//! Bar module driving the desktop wallpaper.
//!
//! The wallpaper is its own thing on the desktop, changed on its own and for
//! its own reasons, so it gets its own entry rather than riding along inside
//! the theme. A theme is a whole look; a wallpaper is one picture inside it,
//! and the two are asked for at different moments.
//!
//! Pressing the entry moves forward, the right button moves back, and the
//! middle button opens the picker: a grid of the theme's wallpapers drawn
//! from the square thumbnails `HyDE` already keeps in its cache, one press on
//! a tile and the desktop wears it.
//!
//! One folder, four rooms: [`listing`] reads the theme's wallpapers and
//! decodes their thumbnails, [`state`] folds messages in and dispatches
//! commands, [`view`] draws the picker grid and [`module`] wires the module
//! to the bar. The root holds the state the rooms share.

pub mod current;
pub(crate) mod listing;
mod module;
mod state;
mod view;

/// One wallpaper ready to draw: path to set, pixels to show.
///
/// The thumbnail is decoded here rather than handed to the renderer as a
/// path: the renderer stays silent about a file it cannot read, and a grid
/// of invisible tiles is exactly the bug this replaced. Decoded pixels
/// either exist or the tile is not offered.
#[derive(Debug, Clone)]
pub struct WallpaperEntry {
    /// Full path of the picture, what a set command takes.
    pub path:  String,
    /// Decoded square thumbnail.
    thumbnail: iced::widget::image::Handle
}

/// Choice made in the wallpaper module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Ask `HyDE` for the next wallpaper of the theme in force.
    Next,
    /// Ask `HyDE` for the previous wallpaper of the theme in force.
    Previous,
    /// Report that the wallpaper change has ended.
    Changed {
        /// Why the desktop refused, if it did.
        failure: Option<String>
    },
    /// Deliver the wallpapers of the theme in force to the picker.
    Listed(Vec<WallpaperEntry>),
    /// Advance the loading indicator by one frame.
    Tick,
    /// Ask the desktop to wear the picture at the given path.
    Pick(String)
}

/// State of the wallpaper module.
///
/// The desktop owns the wallpaper and its order, so the module keeps nothing of
/// its own: it asks, and the desktop does the rest.
#[derive(Debug, Clone, Default)]
pub struct Wallpaper {
    /// Wallpapers of the theme in force, while the picker shows them.
    entries: Vec<WallpaperEntry>,
    /// Whether a listing is being read right now.
    ///
    /// While it is, the bar entry itself spins: the press was taken and
    /// the pictures are on their way, said in place without a window of
    /// placeholder text jumping in first.
    loading: bool,
    /// Frame the loading indicator is on.
    spinner: crate::modules::themes::Spinner
}

impl Wallpaper {
    /// Builds the module.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the picker has nothing to show yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether a listing is being read right now.
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        self.loading
    }
}
