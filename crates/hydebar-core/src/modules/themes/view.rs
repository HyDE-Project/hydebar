//! Menu of the theme module: the desktop's look, and every way of changing
//! it.
//!
//! The menu is the only place the bar draws any of this. It states what the
//! desktop is on and lists what it could be on. The settings window used to
//! hold a page of its own and no longer does, so there is one list, one set
//! of chip states and one wait indicator rather than two that have to
//! be kept in step.

mod gallery_cards;
mod installed_cards;
mod sizing;
mod status;
mod toolbar;

use std::collections::HashMap;

use hydebar_proto::{hyde_state::HydeState, theme_source::ThemeSwatch};
use iced::Element;
pub(super) use sizing::{desired_height, desired_width};

use super::{Message, Spinner};
use crate::components::page::widgets::{page, rows as row_stack, section, status_row};

/// Height of a card's action row, in multiples of the control size.
const ACTIONS_ROW_EM: f32 = 1.8;

/// Height of the update-all row, in multiples of the control size.
const UPDATE_ALL_ROW_EM: f32 = 2.0;

/// Theme chips a row is sized to hold.
///
/// The menu has to name a width before it knows how the chips wrap, and
/// this is the number that keeps a typical `HyDE` install to a handful of
/// rows without making the menu wider than it needs to be.
const THEMES_PER_ROW: f32 = 3.0;

/// Title of the section listing the installed themes.
const THEMES: &str = "Installed";

/// Title of the section listing the gallery the desktop can install from.
const GALLERY: &str = "Available";

/// Label of the row naming what the desktop is on.
const ACTIVE: &str = "Active";

/// Shown in place of the theme name while `HyDE` has recorded none.
const UNKNOWN: &str = "unknown";

/// Placed between the theme in force and the one being switched to.
///
/// A switch takes seconds, and for most of them the desktop is still on the
/// old theme; naming both is the only honest thing the menu can draw,
/// and it is also what tells the user the press was taken.
const SWITCHING_TO: &str = " \u{2192} ";

/// Shown in place of the theme list while none are installed.
const NO_THEMES: &str = "no HyDE themes found on this machine";

/// Rows the menu spends on the row naming the active theme.
const ACTIVE_ROWS: f32 = 1.0;

/// Sections the menu draws.
const SECTION_COUNT: f32 = 1.0;

/// Renders the menu against the desktop state last read from disk.
///
/// `available_width` is how wide the menu may draw, so the chips wrap
/// inside it instead of running past its edge.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the menu renders every piece of module state the bar holds"
)]
pub(super) fn view<'a>(
    state: &HydeState,
    swatches: &HashMap<String, ThemeSwatch>,
    screenshots: &HashMap<String, std::path::PathBuf>,
    switching: Option<&str>,
    catalogue: &[super::gallery::GalleryTheme],
    offered: &[String],
    catalogue_index: &HashMap<String, usize>,
    author: Option<&str>,
    installing: Option<&str>,
    updating: Option<&Option<String>>,
    list_layout: bool,
    spinner: Spinner,
    opacity: f32,
    font_size: f32,
    available_width: f32
) -> Element<'a, Message> {
    let busy_glyph = if switching.is_some() || updating.is_some() {
        Some(spinner.glyph())
    } else {
        None
    };
    let active = row_stack(font_size).push(status_row(
        ACTIVE,
        status::active_label(state, switching),
        busy_glyph,
        font_size
    ));

    let cell = sizing::shared_cell(state, offered, list_layout, font_size, available_width);

    let mut window = page(font_size).push(active).push(section(
        THEMES,
        installed_cards::themes(
            state,
            swatches,
            screenshots,
            switching,
            updating,
            installing,
            catalogue,
            catalogue_index,
            author,
            spinner,
            opacity,
            font_size,
            available_width,
            cell,
            list_layout
        ),
        font_size
    ));

    if !offered.is_empty() {
        window = window.push(section(
            GALLERY,
            gallery_cards::offer(
                offered,
                catalogue,
                catalogue_index,
                screenshots,
                author,
                switching,
                installing,
                spinner,
                opacity,
                font_size,
                available_width,
                cell,
                list_layout
            ),
            font_size
        ));
    }

    window.into()
}

/// Whether two spellings name one theme.
///
/// Dashes and underscores stand in for spaces across the gallery, its
/// branches and the installed directories, and case drifts between them.
#[cfg(test)]
fn same_theme(a: &str, b: &str) -> bool {
    canonical(a) == canonical(b)
}

/// One spelling for a theme name, whatever surface wrote it down.
pub(super) fn canonical(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '-' | '_' => ' ',
            other => other.to_ascii_lowercase()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::same_theme;

    #[test]
    fn one_theme_is_recognised_under_every_spelling() {
        assert!(same_theme("Catppuccin Mocha", "Catppuccin-Mocha"));
        assert!(same_theme("one_dark", "One Dark"));
        assert!(!same_theme("Tokyo Night", "Nordic Blue"));
    }
}
