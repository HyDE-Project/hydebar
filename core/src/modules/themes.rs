//! Bar module driving the desktop theme.
//!
//! Everything about the look of the desktop lives here: the installed themes,
//! the one in force, the facts `HyDE` reports about the wallpaper, and the two
//! actions that change either — switching the theme and asking for the next
//! wallpaper. The settings window is about the bar and holds none of it, so
//! there is one surface to look at rather than two that have to agree.
//!
//! The themes belong to the [HyDE Project](https://github.com/HyDE-Project)
//! rather than to the bar, so nothing chosen here is written into the bar's own
//! configuration file: pressing a theme asks `HyDE`'s own switcher to run, and
//! the desktop — the bar included — follows. What the module shows is read back
//! from `HyDE`'s state, so it reports the desktop as it is even when the change
//! came from a keybinding rather than from here.
//!
//! This is also the one place that knows a switch is running. A `HyDE` switch
//! rewrites the wallpaper, the palette and every generated stylesheet, and
//! takes seconds doing it; the module holds that wait, refuses a second switch
//! on top of it, and owns the indicator its menu and its bar entry draw.

mod dispatch;
mod gallery;
mod install;
mod message;
mod progress;
mod removal;
mod state;
mod switching;
mod updates;
mod view;
mod window;

use std::collections::HashMap;

use hydebar_proto::{hyde_state, theme_source::ThemeSwatch};
use iced::Element;
pub use message::Message;
pub use progress::{FRAME_INTERVAL, Spinner};

use super::OnModulePress;
use crate::{
    components::icons::{IconTheme, Icons, icon, icon_raw_sized},
    menu::MenuType
};

/// Gap between the bar entry and the indicator of a running switch, in pixels.
///
/// Narrow on purpose: the two glyphs have to read as one entry that is busy
/// rather than as two entries that happen to sit next to each other.
const INDICATOR_GAP: f32 = 4.0;

/// Bar entry listing the installed desktop themes.
#[derive(Default, Debug, Clone)]
pub struct Themes {
    /// Desktop state the module draws.
    ///
    /// Kept here rather than read while rendering: the menu is redrawn on every
    /// frame of the open animation, and reading two files that often would put
    /// the filesystem in the draw path.
    hyde:            hyde_state::HydeState,
    /// The colours each theme announces itself with, by theme name.
    ///
    /// Loaded off the update path — see [`Themes::load_swatches`] — and kept
    /// so the menu can paint every chip in the colours of the theme it stands
    /// for. A theme without an entry is painted like any other control.
    swatches:        HashMap<String, ThemeSwatch>,
    /// Screenshots of the desktop wearing each theme, from the local gallery
    /// database, by canonical name.
    screenshots:     HashMap<String, std::path::PathBuf>,
    /// Theme a switch is running for, while one is.
    switching:       Option<String>,
    /// Theme asked for while another switch was still running.
    ///
    /// A press mid-switch used to vanish without a word; now it waits its
    /// turn and runs the moment the desktop settles.
    pending:         Option<String>,
    /// The upstream catalogue, once the menu has loaded it.
    catalogue:       Vec<gallery::GalleryTheme>,
    /// Name the machine's git identity signs work with, once known.
    author:          Option<String>,
    /// Theme an install is running for, while one is.
    installing:      Option<String>,
    /// Theme a removal is deleting, while one is.
    removing:        Option<String>,
    /// Theme an update is fetching, while one is; `None` name means all.
    #[expect(
        clippy::option_option,
        reason = "the outer layer marks a running fetch, the inner one tells one theme from all"
    )]
    updating:        Option<Option<String>>,
    /// Whether the window lays cards out as one column instead of a grid.
    list_layout:     bool,
    /// Frame the indicator of a running switch is on.
    ///
    /// Advanced on a tick rather than derived from a clock read while drawing,
    /// so what the bar shows is a function of the state it holds and can be
    /// checked without one.
    spinner:         Spinner,
    /// Gallery names not installed, refreshed when either side changes.
    ///
    /// The comparison normalises every spelling; done per frame it was
    /// thousands of small strings, done here it is none.
    offered:         Vec<String>,
    /// Catalogue positions by canonical name, refreshed with the catalogue.
    catalogue_index: HashMap<String, usize>
}

impl Themes {
    /// Creates the module against the desktop state on disk.
    #[must_use]
    pub fn new() -> Self {
        let mut themes = Self {
            hyde:            hyde_state::load(),
            swatches:        HashMap::new(),
            screenshots:     HashMap::new(),
            switching:       None,
            pending:         None,
            catalogue:       Vec::new(),
            author:          None,
            installing:      None,
            updating:        None,
            list_layout:     false,
            spinner:         Spinner::default(),
            removing:        None,
            offered:         Vec::new(),
            catalogue_index: HashMap::new()
        };
        themes.reindex();

        themes
    }
}

impl Themes {
    /// The bar entry, with the indicator of a running switch beside it.
    ///
    /// The indicator belongs on the bar and not only in the menu because the
    /// menu is not where the user is looking: a `HyDE` switch repaints the
    /// whole desktop, a menu open over it is dismissed or redrawn along
    /// with it, and the bar is the one surface that is certainly still on
    /// screen. The module icon stays where it was so the entry is still
    /// recognisable as the one that was pressed.
    ///
    /// Rendered by the module itself, so the bar layer holds no theme drawing
    /// of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static
    {
        let entry: Element<'static, M> = if self.is_waiting() {
            iced::widget::Row::new()
                .push(icon(icons, Icons::Themes))
                .push(icon_raw_sized(
                    self.spinner.glyph().to_owned(),
                    icons.size()
                ))
                .spacing(INDICATOR_GAP)
                .align_y(iced::Alignment::Center)
                .into()
        } else {
            icon(icons, Icons::Themes).into()
        };

        Some((entry, Some(OnModulePress::ToggleMenu(MenuType::Themes))))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::menu::MenuType;

    #[test]
    fn the_theme_glyph_is_always_drawn_and_opens_the_gallery() {
        let themes = Themes::default();

        let (_, press) = themes
            .bar_view::<()>(&IconTheme::default())
            .expect("the theme entry draws");

        assert!(matches!(
            press,
            Some(OnModulePress::ToggleMenu(MenuType::Themes))
        ));
    }
}
