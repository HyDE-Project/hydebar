//! HyDE page of the settings window.
//!
//! The page is about the desktop rather than about the bar, so nothing on it is
//! written into the bar's configuration file. What it shows is read back from
//! HyDE's own state, so it reports the desktop as it is even when the change
//! came from a keybinding rather than from the bar.
//!
//! The themes themselves are not here. They live in the theme module on the
//! bar, which is one press away wherever the user happens to be looking, while
//! this window is three. Listing them in both places would have meant two grids
//! reading two copies of the same state, so the page names the theme in force,
//! reports the desktop facts HyDE owns but takes no instruction for, and keeps
//! the one action that is about the desktop as a whole rather than about a
//! theme: asking for the next wallpaper.
//!
//! Facts HyDE owns but does not take an instruction for, such as whether the
//! colours are pulled from the wallpaper, are shown as plain text: a control
//! that cannot act is worse than a line that simply states the truth.

use hydebar_proto::hyde_state::HydeState;
use iced::Element;

use crate::{
    components::page::{
        metrics::{button_width, indicator_width, status_row_width, text_width},
        style,
        widgets::{choice_button, group, note, page, rows as row_stack, section, status_row}
    },
    modules::{
        settings::Message,
        themes::{Spinner, active_label}
    }
};

/// Label of the button asking HyDE for another wallpaper.
const NEXT_WALLPAPER: &str = "next wallpaper";

/// Title of the section reporting what the desktop is set to.
const DESKTOP: &str = "Desktop";

/// Title of the section holding the wallpaper action.
const WALLPAPER: &str = "Wallpaper";

/// Shown in place of the theme name while HyDE has recorded none.
const UNKNOWN: &str = "unknown";

/// Says where the themes went once they left this page.
const THEMES_MOVED: &str = "themes are chosen from the Themes module on the bar";

/// Labels of the rows this page reports a value on.
///
/// Written down once so the width the page asks for and the label column it is
/// held to are measured against the very strings that are drawn.
const STATUS_LABELS: [&str; 3] = ["Active theme", "Wallpaper colours", "Shader"];

/// Sections this page draws.
const SECTION_COUNT: f32 = 2.0;

/// Rows the desktop section spends on the note under its values.
const NOTE_ROWS: f32 = 1.0;

/// Rows the wallpaper section holds.
const WALLPAPER_ROWS: f32 = 1.0;

/// Renders the HyDE page against the desktop state last read from disk.
///
/// `switching` and `spinner` come from the theme module, which is the one place
/// a running switch is held: the window and the module menu therefore mark the
/// same wait with the same glyph on the same frame, rather than each keeping a
/// notion of its own.
pub(super) fn view<'a>(
    state: &HydeState,
    switching: Option<&str>,
    spinner: Spinner,
    opacity: f32,
    font_size: f32
) -> Element<'a, Message> {
    let desktop = row_stack(font_size)
        .push(status_row(
            STATUS_LABELS[0],
            active_label(state, switching),
            switching.map(|_| spinner.glyph()),
            font_size
        ))
        .push(status_row(
            STATUS_LABELS[1],
            switch_label(state.wallpaper_colors).to_owned(),
            None,
            font_size
        ))
        .push(status_row(
            STATUS_LABELS[2],
            state.shader.clone().unwrap_or_else(|| UNKNOWN.to_owned()),
            None,
            font_size
        ))
        .push(note(THEMES_MOVED, font_size));

    let wallpaper = group(font_size).push(choice_button(
        NEXT_WALLPAPER,
        Message::NextHydeWallpaper,
        false,
        font_size,
        opacity
    ));

    page(font_size)
        .push(section(DESKTOP, desktop.into(), font_size))
        .push(section(WALLPAPER, wallpaper.into(), font_size))
        .into()
}

/// Names the state of a switch the page reports but does not operate.
fn switch_label(enabled: bool) -> &'static str {
    if enabled { "On" } else { "Off" }
}

/// Rows this page draws, its section headings counted in.
#[must_use]
pub(super) fn rows() -> f32 {
    SECTION_COUNT * style::SECTION_TITLE_ROWS
        + STATUS_LABELS.len() as f32
        + NOTE_ROWS
        + WALLPAPER_ROWS
}

/// Longest line of this page, which is how wide the window has to be.
///
/// Room for the live indicator is reserved whether one is showing or not. It
/// costs a glyph of width, and it buys a window that does not jump wider the
/// instant a switch starts somewhere else on the bar.
#[must_use]
pub(super) fn desired_width(state: &HydeState, switching: Option<&str>, font_size: f32) -> f32 {
    let statuses = [
        active_label(state, switching),
        switch_label(state.wallpaper_colors).to_owned(),
        state.shader.clone().unwrap_or_else(|| UNKNOWN.to_owned())
    ]
    .iter()
    .map(|value| status_row_width(value, font_size))
    .fold(0.0_f32, f32::max)
        + indicator_width(font_size);

    let control = style::control_size(font_size);
    let moved = text_width(THEMES_MOVED, style::caption_size(font_size));
    let wallpaper = button_width(NEXT_WALLPAPER, control);

    statuses.max(moved).max(wallpaper)
}

/// Height this page needs.
#[must_use]
pub(super) fn desired_height(font_size: f32) -> f32 {
    style::page_height(rows(), font_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(themes: &[&str], active: Option<&str>) -> HydeState {
        HydeState {
            theme:            active.map(str::to_owned),
            themes:           themes.iter().map(|name| (*name).to_owned()).collect(),
            wallpaper_colors: true,
            shader:           Some("wallbash".to_owned())
        }
    }

    #[test]
    fn a_switch_is_reported_as_on_or_off() {
        assert_eq!(switch_label(true), "On");
        assert_eq!(switch_label(false), "Off");
    }

    #[test]
    fn a_page_without_themes_still_asks_for_a_width() {
        assert!(desired_width(&HydeState::default(), None, 16.0) > 0.0);
    }

    /// The theme grid moved to the module, so the page no longer grows with the
    /// number of themes installed; it is one fixed set of rows.
    #[test]
    fn the_page_is_the_same_size_however_many_themes_are_installed() {
        let few = state(&["Nord"], Some("Nord"));
        let many = state(
            &[
                "Nord",
                "Mocha",
                "Latte",
                "Decay Green",
                "Edge Runner",
                "Synth Wave"
            ],
            Some("Nord")
        );

        assert_eq!(
            desired_width(&few, None, 16.0),
            desired_width(&many, None, 16.0)
        );
        assert_eq!(desired_height(16.0), desired_height(16.0));
    }

    #[test]
    fn a_running_switch_widens_the_page_enough_to_draw_it() {
        let state = state(&["Nord", "An Extremely Long Theme Name"], Some("Nord"));
        let pending = Some("An Extremely Long Theme Name");

        assert!(
            desired_width(&state, pending, 16.0)
                >= status_row_width(&active_label(&state, pending), 16.0)
        );
    }

    #[test]
    fn every_row_label_fits_the_shared_label_column() {
        let font_size = 16.0;

        for label in STATUS_LABELS {
            assert!(
                text_width(label, font_size) <= style::label_width(font_size),
                "{label} overflows the label column"
            );
        }
    }

    #[test]
    fn the_page_reserves_a_heading_for_every_section_it_draws() {
        assert_eq!(
            rows(),
            SECTION_COUNT + STATUS_LABELS.len() as f32 + NOTE_ROWS + WALLPAPER_ROWS
        );
    }

    #[test]
    fn the_page_height_follows_the_shared_row_pitch() {
        let font_size = 16.0;

        assert_eq!(
            desired_height(font_size),
            style::page_height(rows(), font_size)
        );
    }

    #[test]
    fn the_page_says_where_the_themes_went() {
        assert!(THEMES_MOVED.contains("Themes"));
    }
}
