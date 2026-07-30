//! Menu of the theme module: the desktop's look, and every way of changing it.
//!
//! The menu is the only place the bar draws any of this. It states what the
//! desktop is on and lists what it could be on. The settings window used to
//! hold a page of its own and no longer does, so there is one list, one set of
//! chip states and one wait indicator rather than two that have to be kept in
//! step.

use std::collections::HashMap;

use hydebar_proto::{
    hyde_state::HydeState,
    theme_source::{Rgba, ThemeSwatch}
};
use iced::Element;

use super::{Message, Spinner};
use crate::components::page::{
    metrics::{
        chip_cell_width, chip_width, indicator_width, status_row_width, text_width,
        wrap_chips_into_rows
    },
    style,
    widgets::{
        ChipPaint, ThemeChip, grid, group, note, page, rows as row_stack, section, status_row,
        theme_chip
    }
};

/// Theme chips a row is sized to hold.
///
/// The menu has to name a width before it knows how the chips wrap, and this is
/// the number that keeps a typical HyDE install to a handful of rows without
/// making the menu wider than it needs to be.
const THEMES_PER_ROW: f32 = 3.0;

/// Title of the section listing the installed themes.
const THEMES: &str = "Themes";

/// Label of the row naming what the desktop is on.
const ACTIVE: &str = "Active";

/// Shown in place of the theme name while HyDE has recorded none.
const UNKNOWN: &str = "unknown";

/// Placed between the theme in force and the one being switched to.
///
/// A switch takes seconds, and for most of them the desktop is still on the old
/// theme; naming both is the only honest thing the menu can draw, and it is
/// also what tells the user the press was taken.
const SWITCHING_TO: &str = " \u{2192} ";

/// Shown in place of the theme list while none are installed.
const NO_THEMES: &str = "no HyDE themes found on this machine";

/// Rows the menu spends on the row naming the active theme.
const ACTIVE_ROWS: f32 = 1.0;

/// Sections the menu draws.
const SECTION_COUNT: f32 = 1.0;

/// Renders the menu against the desktop state last read from disk.
///
/// `available_width` is how wide the menu may draw, so the chips wrap inside it
/// instead of running past its edge.
#[must_use]
pub(super) fn view<'a>(
    state: &HydeState,
    swatches: &HashMap<String, ThemeSwatch>,
    switching: Option<&str>,
    spinner: Spinner,
    opacity: f32,
    font_size: f32,
    available_width: f32
) -> Element<'a, Message> {
    let active = row_stack(font_size).push(status_row(
        ACTIVE,
        active_label(state, switching),
        switching.map(|_| spinner.glyph()),
        font_size
    ));

    page(font_size)
        .push(active)
        .push(section(
            THEMES,
            themes(
                state,
                swatches,
                switching,
                spinner,
                opacity,
                font_size,
                available_width
            ),
            font_size
        ))
        .into()
}

/// Renders the installed themes as a grid of chips.
///
/// The theme in force is drawn as picked, so the grid doubles as the answer to
/// "which one am I on" without the menu repeating the name twice.
///
/// While a switch runs the grid stops being a set of choices: the theme being
/// applied is the only one lit, and every other chip is dimmed and carries no
/// press at all. A second switch started on top of the first would race it over
/// the state file, the wallpaper cache and every generated stylesheet, and the
/// module refuses one anyway — a grid that still looked pressable would only be
/// hiding that refusal until after the click.
fn themes<'a>(
    state: &HydeState,
    swatches: &HashMap<String, ThemeSwatch>,
    switching: Option<&str>,
    spinner: Spinner,
    opacity: f32,
    font_size: f32,
    available_width: f32
) -> Element<'a, Message> {
    if state.themes.is_empty() {
        return note(NO_THEMES, font_size);
    }

    let gap = style::group_gap(font_size);
    let cell = chip_cell_width(&state.themes, font_size);
    let mut block = grid(font_size);

    for indices in wrap_chips_into_rows(&state.themes, available_width, font_size, gap) {
        let mut row = group(font_size);

        for index in indices {
            let name = &state.themes[index];

            row = row.push(theme_chip(
                name.clone(),
                Message::Switch(name.clone()),
                chip_state(state, switching, spinner, name),
                font_size,
                opacity,
                cell,
                swatches.get(name).map(chip_paint)
            ));
        }

        block = block.push(row);
    }

    block.into()
}

/// Restates a theme's swatch in the colours the renderer paints with.
fn chip_paint(swatch: &ThemeSwatch) -> ChipPaint {
    ChipPaint {
        background: colour(swatch.background),
        text:       colour(swatch.text),
        accent:     colour(swatch.accent)
    }
}

/// One palette colour, as the renderer spells it.
fn colour(rgba: Rgba) -> iced::Color {
    iced::Color::from_rgba8(rgba.r, rgba.g, rgba.b, rgba.a)
}

/// What the chip of `name` stands for, given what the desktop reports and what
/// the bar is waiting on.
///
/// The theme being applied wins over the theme in force, because HyDE reports
/// the new name in its state file long before the switch is anywhere near done:
/// a grid that only looked at the state file would light the new chip within a
/// moment of the press and then sit completely still for the seconds that
/// actually matter.
fn chip_state(
    state: &HydeState,
    switching: Option<&str>,
    spinner: Spinner,
    name: &str
) -> ThemeChip {
    match switching {
        Some(pending) if pending == name => ThemeChip::Applying(spinner),
        Some(_) => ThemeChip::Blocked,
        None if state.is_active(name) => ThemeChip::Active,
        None => ThemeChip::Idle
    }
}

/// Renders what the desktop is on, and what it is on its way to.
///
/// The theme in force always comes from HyDE's own state file; a theme the bar
/// asked for is drawn beside it rather than in its place, because until the
/// switch has finished the desktop is still on the old one and a menu that
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

/// Rows this menu draws when laid out `available_width` wide, its heading
/// counted in.
#[must_use]
pub(super) fn rows(state: &HydeState, font_size: f32, available_width: f32) -> f32 {
    SECTION_COUNT * style::SECTION_TITLE_ROWS
        + ACTIVE_ROWS
        + theme_rows(state, font_size, available_width)
}

/// Longest line the active row can ever grow to.
///
/// While a switch runs the row names both themes, `active → pending`, and any
/// installed theme can be the pending one. Reserving for the longest of those
/// lines up front is what keeps the menu the same size through a press: a
/// window that jumps wider the instant it is pressed jumps at the one moment
/// the user is looking straight at it.
fn widest_active_row(state: &HydeState, switching: Option<&str>, font_size: f32) -> f32 {
    let shown = status_row_width(&active_label(state, switching), font_size);
    let active = state.theme.as_deref().unwrap_or(UNKNOWN);

    state
        .themes
        .iter()
        .filter(|name| !state.is_active(name))
        .map(|name| status_row_width(&format!("{active}{SWITCHING_TO}{name}"), font_size))
        .fold(shown, f32::max)
}

/// Longest line of this menu, which is how wide it has to be.
///
/// The grid is measured as a row of the widest themes rather than as the whole
/// list: it wraps into whatever width the menu settles on, and sizing the menu
/// to hold every theme side by side would make it far wider than the screen.
///
/// Room for the live indicator and for the longest possible switch line is
/// reserved whether a switch is running or not, so starting one moves
/// nothing — see [`widest_active_row`].
#[must_use]
pub(super) fn desired_width(state: &HydeState, switching: Option<&str>, font_size: f32) -> f32 {
    let active = widest_active_row(state, switching, font_size) + indicator_width(font_size);

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

    active.max(grid)
}

/// Height this menu needs when drawn `available_width` wide.
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
    fn a_menu_that_is_not_switching_names_the_theme_in_force() {
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
    /// over, so the menu has to stop drawing an arrow that points at the theme
    /// it already reports.
    #[test]
    fn a_switch_the_state_file_already_reports_is_named_only_once() {
        let state = state(&["Nord", "Mocha"], Some("Mocha"));

        assert_eq!(active_label(&state, Some("Mocha")), "Mocha");
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
    fn a_menu_nobody_is_waiting_on_marks_the_theme_in_force_and_offers_the_rest() {
        let state = state(&["Nord", "Mocha"], Some("Nord"));
        let spinner = Spinner::default();

        assert_eq!(chip_state(&state, None, spinner, "Nord"), ThemeChip::Active);
        assert_eq!(chip_state(&state, None, spinner, "Mocha"), ThemeChip::Idle);
    }

    #[test]
    fn a_running_switch_marks_the_theme_it_is_applying() {
        let state = state(&["Nord", "Mocha"], Some("Nord"));
        let spinner = Spinner::default();

        assert_eq!(
            chip_state(&state, Some("Mocha"), spinner, "Mocha"),
            ThemeChip::Applying(spinner)
        );
    }

    #[test]
    fn no_chip_can_start_a_second_switch_while_one_runs() {
        let state = state(&["Nord", "Mocha", "Latte"], Some("Nord"));
        let spinner = Spinner::default();

        for name in &state.themes {
            assert!(
                !chip_state(&state, Some("Mocha"), spinner, name).is_pressable(),
                "{name}"
            );
        }
    }

    /// The one case the state file cannot settle: HyDE records the new theme
    /// within a moment of the press and the switch runs for seconds afterwards,
    /// so a grid that trusted the file would go still exactly when the user is
    /// waiting hardest.
    #[test]
    fn the_theme_being_applied_stays_marked_once_the_state_file_names_it() {
        let state = state(&["Nord", "Mocha"], Some("Mocha"));
        let spinner = Spinner::default();

        assert_eq!(
            chip_state(&state, Some("Mocha"), spinner, "Mocha"),
            ThemeChip::Applying(spinner)
        );
    }

    #[test]
    fn the_marked_chip_keeps_moving_as_the_indicator_advances() {
        let state = state(&["Nord", "Mocha"], Some("Nord"));
        let mut spinner = Spinner::default();
        let first = chip_state(&state, Some("Mocha"), spinner, "Mocha");

        spinner.advance();

        assert_ne!(chip_state(&state, Some("Mocha"), spinner, "Mocha"), first);
    }

    #[test]
    fn a_menu_without_themes_still_asks_for_a_width() {
        assert!(desired_width(&HydeState::default(), None, 16.0) > 0.0);
    }

    #[test]
    fn a_menu_without_themes_reserves_one_row_for_the_notice() {
        assert_eq!(theme_rows(&HydeState::default(), 16.0, 400.0), 1.0);
    }

    #[test]
    fn a_long_theme_name_widens_the_menu() {
        let short = state(&["Nord"], Some("Nord"));
        let long = state(&["An Extremely Long Theme Name"], Some("Nord"));

        assert!(desired_width(&long, None, 16.0) > desired_width(&short, None, 16.0));
    }

    #[test]
    fn more_themes_than_a_row_holds_make_the_menu_taller() {
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
    fn a_menu_showing_an_indicator_is_no_wider_than_one_that_is_not() {
        let state = state(&["Nord", "Mocha"], Some("Nord"));

        assert_eq!(
            desired_width(&state, Some("Nord"), 16.0),
            desired_width(&state, None, 16.0)
        );
    }

    #[test]
    fn the_menu_reserves_the_indicator_before_anything_is_running() {
        let state = state(&["A Theme Long Enough To Set The Width"], None);
        let font_size = 16.0;

        assert!(
            desired_width(&state, None, font_size)
                >= status_row_width(&active_label(&state, None), font_size)
                    + indicator_width(font_size)
        );
    }

    /// Starting a switch must not move the menu: the width already holds the
    /// longest line the active row can become.
    #[test]
    fn starting_a_switch_changes_no_size() {
        let themes = state(
            &["Nord", "A Theme With A Very Long Name Indeed"],
            Some("Nord")
        );
        let font_size = 16.0;

        let resting = desired_width(&themes, None, font_size);
        let switching = desired_width(
            &themes,
            Some("A Theme With A Very Long Name Indeed"),
            font_size
        );

        assert_eq!(resting, switching);
        assert_eq!(
            desired_height(&themes, font_size, resting),
            desired_height(&themes, font_size, switching)
        );
    }

    #[test]
    fn the_menu_reserves_a_row_for_every_line_it_draws() {
        let themes = state(&["Nord"], Some("Nord"));
        let font_size = 16.0;
        let width = desired_width(&themes, None, font_size);

        assert_eq!(
            rows(&themes, font_size, width),
            SECTION_COUNT * style::SECTION_TITLE_ROWS
                + ACTIVE_ROWS
                + theme_rows(&themes, font_size, width)
        );
    }

    #[test]
    fn the_menu_height_follows_the_shared_row_pitch() {
        let font_size = 16.0;
        let themes = state(&["Nord", "Mocha"], Some("Nord"));

        assert_eq!(
            desired_height(&themes, font_size, 400.0),
            style::page_height(rows(&themes, font_size, 400.0), font_size)
        );
    }
}
