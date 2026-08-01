//! Choices made in the theme module, and the reports that answer them.
//!
//! Every press the menu can take and every ending the desktop can announce
//! is one variant here, so the whole conversation between the bar and
//! `HyDE` can be read in one place.

use std::collections::HashMap;

use hydebar_proto::theme_source::ThemeSwatch;

use super::gallery;

/// Choice made in the theme module.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Ask `HyDE` for the next theme in its own order.
    NextTheme,
    /// Ask `HyDE` for the previous theme in its own order.
    PreviousTheme,
    /// Report that a stepped switch has ended.
    Stepped {
        /// Why the desktop refused, if it did.
        failure: Option<String>
    },
    /// Ask `HyDE` to switch the desktop to the named theme.
    Switch(String),
    /// Report that the switch to the named theme has ended.
    ///
    /// Raised by the bar itself once the desktop's own switch has exited, so
    /// the module stops promising a switch that is over and states what the
    /// desktop actually settled on.
    Switched {
        /// Theme the switch was asked for.
        theme:   String,
        /// Why the switch failed, when it did.
        failure: Option<String>
    },
    /// Move the indicator of a running switch on by one frame.
    ///
    /// Raised on a timer for as long as a switch is running, and never
    /// otherwise: the bar has no other reason to redraw itself while it waits
    /// on a desktop script, and a wait nobody can see reads as a press that was
    /// never taken.
    Tick,
    /// Deliver the swatches the themes announce themselves with.
    ///
    /// Raised by the reader the menu starts when it opens; the colours arrive
    /// a beat after the names because reading them hashes every theme's
    /// wallpaper, which is nothing the opening animation should wait on.
    SwatchesLoaded(
        HashMap<String, ThemeSwatch>,
        HashMap<String, std::path::PathBuf>
    ),
    /// Deliver the upstream catalogue the gallery section draws.
    CatalogueLoaded(Vec<gallery::GalleryTheme>, Option<String>),
    /// Install the named theme from the gallery, then switch to it.
    Install(String),
    /// Report that the install of the named theme has ended.
    Installed {
        /// Theme the install was asked for.
        theme:   String,
        /// Why the install failed, when it did.
        failure: Option<String>
    },
    /// Remove the named theme, previously condemned.
    Remove(String),
    /// Report that the removal of the named theme has ended.
    Removed {
        /// Theme the removal was asked for.
        theme:   String,
        /// Why the removal failed, when it did.
        failure: Option<String>
    },
    /// Fetch updates for one installed theme, or all of them.
    Update(Option<String>),
    /// Flip the window between the grid and the single-column layout.
    ToggleLayout,
    /// Report that an update fetch has ended.
    Updated {
        /// Why the fetch failed, when it did.
        failure: Option<String>
    }
}
