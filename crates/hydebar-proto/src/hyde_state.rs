//! Reader for the desktop state published by the [HyDE Project](https://github.com/HyDE-Project).
//!
//! HyDE keeps the choices the user made about the desktop — the theme in force,
//! the wallpaper shader, whether the colours are recoloured from the wallpaper
//! — in a small shell file that every HyDE script sources. The installed themes
//! are simply the directories under the HyDE configuration root.
//!
//! The bar reads both so it can show the desktop as it is rather than as the
//! bar's own configuration remembers it, and so a page of the settings window
//! can offer the themes that actually exist on this machine.
//!
//! Like [`crate::theme_source`], the reader never fails: a missing file, an
//! unreadable directory or a line it cannot understand simply leaves the
//! affected field at its default. A settings page that showed an error where a
//! theme name belongs would be worse than one that shows nothing.
//!
//! The assignment parser lives in [`crate::shell_vars`], the directory listing
//! in [`themes`], and the snapshot with its entry points in [`state`].

mod state;
mod themes;

pub use state::{HydeState, load, load_from};
