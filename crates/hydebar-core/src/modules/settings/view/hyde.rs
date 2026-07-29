//! HyDE page of the settings window.
//!
//! The page is about the desktop rather than about the bar, so nothing on it is
//! written into the bar's configuration file: pressing a theme asks HyDE's own
//! switcher to run, and the desktop — the bar included — follows. What the page
//! shows is read back from HyDE's own state, so it reports the desktop as it is
//! even when the change came from a keybinding rather than from here, and it
//! keeps reporting the old theme for as long as the switch takes rather than
//! the one that was pressed.
//!
//! Facts HyDE owns but does not take an instruction for, such as whether the
//! colours are pulled from the wallpaper, are shown as plain text: a control
//! that cannot act is worse than a line that simply states the truth.

use hydebar_proto::hyde_state::HydeState;
use iced::Element;

use super::{
    metrics::{button_width, chip_width, status_row_width, text_width, wrap_chips_into_rows},
    style,
    widgets::{
        chip, choice_button, grid, group, note, page, rows as row_stack, section, status_row
    }
};
use crate::modules::settings::Message;

/// Theme buttons a row is sized to hold.
///
/// The page has to name a width before it knows how the buttons wrap, and this
/// is the number that keeps a typical HyDE install to a handful of rows without
/// making the window wider than the pages beside it.
const THEMES_PER_ROW: f32 = 3.0;

/// Label of the button asking HyDE for another wallpaper.
const NEXT_WALLPAPER: &str = "next wallpaper";

/// Title of the section reporting what the desktop is set to.
const DESKTOP: &str = "Desktop";

/// Title of the section listing the installed themes.
const THEMES: &str = "Themes";

/// Title of the section holding the wallpaper action.
const WALLPAPER: &str = "Wallpaper";

/// Shown in place of the theme name while HyDE has recorded none.
const UNKNOWN: &str = "unknown";

/// Placed between the theme in force and the one being switched to.
///
/// A switch takes seconds, and for all of them the desktop is still on the old
/// theme; naming both is the only honest thing the page can draw, and it is
/// also what tells the user the press was taken.
const SWITCHING_TO: &str = " \u{2192} ";

/// Shown in place of the theme list while none are installed.
const NO_THEMES: &str = "no HyDE themes found on this machine";

/// Labels of the rows this page reports a value on.
///
/// Written down once so the width the page asks for and the label column it is
/// held to are measured against the very strings that are drawn.
const STATUS_LABELS: [&str; 3] = ["Active theme", "Wallpaper colours", "Shader"];

/// Sections this page draws.
const SECTION_COUNT: f32 = 3.0;

/// Rows the wallpaper section holds.
const WALLPAPER_ROWS: f32 = 1.0;

/// Renders the HyDE page against the desktop state last read from disk.
///
/// `available_width` is how wide the page may draw, so the theme chips wrap
/// inside the window instead of running past its edge.
pub(super) fn view<'a>(
    state: &HydeState,
    switching: Option<&str>,
    opacity: f32,
    font_size: f32,
    available_width: f32
) -> Element<'a, Message> {
    let desktop = row_stack(font_size)
        .push(status_row(
            STATUS_LABELS[0],
            active_label(state, switching),
            font_size
        ))
        .push(status_row(
            STATUS_LABELS[1],
            switch_label(state.wallpaper_colors).to_owned(),
            font_size
        ))
        .push(status_row(
            STATUS_LABELS[2],
            state.shader.clone().unwrap_or_else(|| UNKNOWN.to_owned()),
            font_size
        ));

    let wallpaper = group(font_size).push(choice_button(
        NEXT_WALLPAPER,
        Message::NextHydeWallpaper,
        false,
        font_size,
        opacity
    ));

    page(font_size)
        .push(section(DESKTOP, desktop.into(), font_size))
        .push(section(
            THEMES,
            themes(state, opacity, font_size, available_width),
            font_size
        ))
        .push(section(WALLPAPER, wallpaper.into(), font_size))
        .into()
}

/// Renders the installed themes as a grid of chips.
///
/// The theme in force is drawn as picked, so the grid doubles as the answer to
/// "which one am I on" without the page repeating the name twice.
fn themes<'a>(
    state: &HydeState,
    opacity: f32,
    font_size: f32,
    available_width: f32
) -> Element<'a, Message> {
    if state.themes.is_empty() {
        return note(NO_THEMES, font_size);
    }

    let gap = style::group_gap(font_size);
    let mut block = grid(font_size);

    for indices in wrap_chips_into_rows(&state.themes, available_width, font_size, gap) {
        let mut row = group(font_size);

        for index in indices {
            let name = &state.themes[index];

            row = row.push(chip(
                name.clone(),
                Message::SwitchHydeTheme(name.clone()),
                state.is_active(name),
                font_size,
                opacity
            ));
        }

        block = block.push(row);
    }

    block.into()
}

/// Names the state of a switch the page reports but does not operate.
fn switch_label(enabled: bool) -> &'static str {
    if enabled { "On" } else { "Off" }
}

/// Renders what the desktop is on, and what it is on its way to.
///
/// The theme in force always comes from HyDE's own state file; a theme the bar
/// asked for is drawn beside it rather than in its place, because until the
/// switch has finished the desktop is still on the old one and a page that
/// already named the new one would be reporting something that may yet fail.
fn active_label(state: &HydeState, switching: Option<&str>) -> String {
    let active = state.theme.as_deref().unwrap_or(UNKNOWN);

    match switching {
        Some(pending) if !state.is_active(pending) => format!("{active}{SWITCHING_TO}{pending}"),
        _ => active.to_owned()
    }
}

/// Rows the theme grid fills when laid out `available_width` wide.
fn theme_rows(state: &HydeState, font_size: f32, available_width: f32) -> f32 {
    if state.themes.is_empty() {
        return 1.0;
    }

    wrap_chips_into_rows(
        &state.themes,
        available_width,
        font_size,
        style::group_gap(font_size)
    )
    .len() as f32
}

/// Rows this page draws when laid out `available_width` wide, its section
/// headings counted in.
#[must_use]
pub(super) fn rows(state: &HydeState, font_size: f32, available_width: f32) -> f32 {
    SECTION_COUNT * style::SECTION_TITLE_ROWS
        + STATUS_LABELS.len() as f32
        + theme_rows(state, font_size, available_width)
        + WALLPAPER_ROWS
}

/// Longest line of this page, which is how wide the window has to be.
///
/// The theme grid is measured as a row of the widest themes rather than as the
/// whole list: the grid wraps into whatever width the rest of the page settles
/// on, and sizing the window to hold every theme side by side would make it far
/// wider than the screen.
#[must_use]
pub(super) fn desired_width(state: &HydeState, switching: Option<&str>, font_size: f32) -> f32 {
    let statuses = [
        active_label(state, switching),
        switch_label(state.wallpaper_colors).to_owned(),
        state.shader.clone().unwrap_or_else(|| UNKNOWN.to_owned())
    ]
    .iter()
    .map(|value| status_row_width(value, font_size))
    .fold(0.0_f32, f32::max);

    let control = style::control_size(font_size);
    let gap = style::group_gap(font_size);

    let grid = if state.themes.is_empty() {
        text_width(NO_THEMES, style::caption_size(font_size))
    } else {
        let widest = state
            .themes
            .iter()
            .map(|name| chip_width(name, control))
            .fold(0.0_f32, f32::max);

        widest * THEMES_PER_ROW + gap * (THEMES_PER_ROW - 1.0)
    };

    let wallpaper = button_width(NEXT_WALLPAPER, control);

    statuses.max(grid).max(wallpaper)
}

/// Height this page needs when drawn `available_width` wide.
#[must_use]
pub(super) fn desired_height(state: &HydeState, font_size: f32, available_width: f32) -> f32 {
    style::page_height(rows(state, font_size, available_width), font_size)
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

    #[test]
    fn a_page_that_is_not_switching_names_the_theme_in_force() {
        let state = state(&["Nord", "Mocha"], Some("Nord"));

        assert_eq!(active_label(&state, None), "Nord");
    }

    #[test]
    fn a_running_switch_names_both_themes_and_keeps_the_old_one_first() {
        let state = state(&["Nord", "Mocha"], Some("Nord"));

        let label = active_label(&state, Some("Mocha"));

        assert!(label.starts_with("Nord"), "{label}");
        assert!(label.ends_with("Mocha"), "{label}");
    }

    /// HyDE writes the new name into its state file long before the switch is
    /// over, so the page has to stop drawing an arrow that points at the theme
    /// it already reports.
    #[test]
    fn a_switch_the_state_file_already_reports_is_named_only_once() {
        let state = state(&["Nord", "Mocha"], Some("Mocha"));

        assert_eq!(active_label(&state, Some("Mocha")), "Mocha");
    }

    #[test]
    fn a_running_switch_leaves_the_old_theme_the_active_one() {
        let state = state(&["Nord", "Mocha"], Some("Nord"));

        assert!(state.is_active("Nord"));
        assert!(!state.is_active("Mocha"));
    }

    #[test]
    fn a_desktop_without_a_theme_still_names_the_one_being_switched_to() {
        let state = state(&["Nord"], None);

        assert_eq!(
            active_label(&state, Some("Nord")),
            format!("{UNKNOWN}{SWITCHING_TO}Nord")
        );
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
    fn a_page_without_themes_reserves_one_row_for_the_notice() {
        assert_eq!(theme_rows(&HydeState::default(), 16.0, 400.0), 1.0);
    }

    #[test]
    fn a_long_theme_name_widens_the_page() {
        let short = state(&["Nord"], Some("Nord"));
        let long = state(&["An Extremely Long Theme Name"], Some("Nord"));

        assert!(desired_width(&long, None, 16.0) > desired_width(&short, None, 16.0));
    }

    #[test]
    fn more_themes_than_a_row_holds_make_the_page_taller() {
        let few = state(&["Nord", "Mocha"], Some("Nord"));
        let many = state(
            &[
                "Nord",
                "Mocha",
                "Latte",
                "Decay Green",
                "Edge Runner",
                "Synth Wave",
                "Tokyo Night",
                "Gruvbox Retro"
            ],
            Some("Nord")
        );
        let width = desired_width(&few, None, 16.0);

        assert!(desired_height(&many, 16.0, width) > desired_height(&few, 16.0, width));
    }

    #[test]
    fn every_theme_lands_in_exactly_one_row() {
        let themes = state(
            &["Nord", "Mocha", "Latte", "Decay Green", "Edge Runner"],
            None
        );
        let rows = wrap_chips_into_rows(&themes.themes, 200.0, 16.0, 8.0);

        assert_eq!(
            rows.iter().map(Vec::len).sum::<usize>(),
            themes.themes.len()
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
        let themes = state(&["Nord"], Some("Nord"));

        assert_eq!(
            rows(&themes, 16.0, 400.0),
            SECTION_COUNT + STATUS_LABELS.len() as f32 + 1.0 + WALLPAPER_ROWS
        );
    }

    #[test]
    fn the_page_height_follows_the_shared_row_pitch() {
        let font_size = 16.0;
        let themes = state(&["Nord", "Mocha"], Some("Nord"));

        assert_eq!(
            desired_height(&themes, font_size, 400.0),
            style::page_height(rows(&themes, font_size, 400.0), font_size)
        );
    }
}
