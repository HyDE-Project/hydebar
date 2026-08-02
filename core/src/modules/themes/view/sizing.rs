//! How large the menu asks to be, settled before anything inside it is laid
//! out.
//!
//! The compositor is told the surface size up front, so every length here is
//! computed from the state alone: how the cards wrap, how many rows the menu
//! spends, and the widest line the active row can ever grow to.

use hydebar_proto::hyde_state::HydeState;

use super::{
    ACTIONS_ROW_EM, ACTIVE_ROWS, NO_THEMES, SECTION_COUNT, SWITCHING_TO, THEMES_PER_ROW, UNKNOWN,
    UPDATE_ALL_ROW_EM, status::active_label
};
use crate::components::page::{
    metrics::{
        chip_cell_width, chip_width, indicator_width, status_row_width, text_width,
        wrap_chips_into_rows
    },
    style, widgets
};

/// One card width for both sections, whatever the layout.
///
/// The two grids used to size their cells from their own names and came
/// out unequal; sizing from every name at once is what makes an installed
/// card and an available one the same card.
pub(super) fn shared_cell(
    state: &HydeState,
    offered: &[String],
    list_layout: bool,
    font_size: f32,
    available_width: f32
) -> f32 {
    if list_layout {
        return available_width;
    }

    let mut names = state.themes.clone();
    names.extend(offered.iter().cloned());

    chip_cell_width(&names, font_size)
}

/// Rows of card indices for the layout in force.
pub(super) fn card_rows(
    names: &[String],
    list_layout: bool,
    font_size: f32,
    available_width: f32
) -> Vec<Vec<usize>> {
    if list_layout {
        (0..names.len()).map(|index| vec![index]).collect()
    } else {
        wrap_chips_into_rows(
            names,
            available_width,
            font_size,
            style::group_gap(font_size)
        )
    }
}

/// Rows the theme grid fills when laid out `available_width` wide.
fn theme_rows(state: &HydeState, font_size: f32, available_width: f32) -> f32 {
    theme_rows_in(state, false, font_size, available_width)
}

/// Rows the installed grid fills in the layout in force.
#[expect(
    clippy::cast_precision_loss,
    reason = "row and theme counts are small, fit f32 exactly"
)]
fn theme_rows_in(
    state: &HydeState,
    list_layout: bool,
    font_size: f32,
    available_width: f32
) -> f32 {
    if list_layout {
        return state.themes.len().max(1) as f32;
    }

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
    SECTION_COUNT.mul_add(style::SECTION_TITLE_ROWS, ACTIVE_ROWS)
        + theme_rows(state, font_size, available_width)
}

/// Longest line the active row can ever grow to.
///
/// While a switch runs the row names both themes, `active → pending`, and
/// any installed theme can be the pending one. Reserving for the
/// longest of those lines up front is what keeps the menu the same size
/// through a press: a window that jumps wider the instant it is pressed
/// jumps at the one moment the user is looking straight at it.
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
/// The grid is measured as a row of the widest themes rather than as the
/// whole list: it wraps into whatever width the menu settles on, and
/// sizing the menu to hold every theme side by side would make it far
/// wider than the screen.
///
/// Room for the live indicator and for the longest possible switch line is
/// reserved whether a switch is running or not, so starting one moves
/// nothing — see [`widest_active_row`].
#[must_use]
pub(in crate::modules::themes) fn desired_width(
    state: &HydeState,
    switching: Option<&str>,
    font_size: f32
) -> f32 {
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

        gap.mul_add(THEMES_PER_ROW - 1.0, widest * THEMES_PER_ROW)
    };

    active.max(grid)
}

/// Height this menu needs when drawn `available_width` wide.
///
/// A painted chip is taller than a plain one by its row of palette dots, so
/// every grid row adds that height on top of the shared row pitch — an
/// estimate without it would clip the last row of themes.
#[must_use]
pub(in crate::modules::themes) fn desired_height(
    state: &HydeState,
    offered: &[String],
    list_layout: bool,
    font_size: f32,
    available_width: f32
) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "row counts are small, fit f32 exactly"
    )]
    let offered_rows = if offered.is_empty() {
        0.0
    } else {
        card_rows(offered, list_layout, font_size, available_width).len() as f32
    };
    let offered_sections = if offered.is_empty() { 0.0 } else { 1.0 };

    let chip_rows = theme_rows_in(state, list_layout, font_size, available_width) + offered_rows;
    let control = style::control_size(font_size);
    let actions = UPDATE_ALL_ROW_EM.mul_add(control, chip_rows * ACTIONS_ROW_EM * control);
    let dots = (chip_rows * widgets::DOT_ROW_EM).mul_add(control, actions);

    style::page_height(
        rows(state, font_size, available_width)
            + offered_rows
            + offered_sections * style::SECTION_TITLE_ROWS,
        font_size
    ) + dots
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, clippy::suboptimal_flops)]

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

        assert!(
            desired_height(&many, &[], false, 16.0, width)
                > desired_height(&few, &[], false, 16.0, width)
        );
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

    /// Starting a switch must not move the menu: the width already holds
    /// the longest line the active row can become.
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
            desired_height(&themes, &[], false, font_size, resting),
            desired_height(&themes, &[], false, font_size, switching)
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
    fn the_menu_height_counts_dots_actions_and_the_update_row() {
        let font_size = 16.0;
        let themes = state(&["Nord", "Mocha"], Some("Nord"));
        let control = style::control_size(font_size);
        let chip_rows = theme_rows(&themes, font_size, 400.0);

        assert_eq!(
            desired_height(&themes, &[], false, font_size, 400.0),
            style::page_height(rows(&themes, font_size, 400.0), font_size)
                + chip_rows * widgets::DOT_ROW_EM * control
                + chip_rows * ACTIONS_ROW_EM * control
                + UPDATE_ALL_ROW_EM * control
        );
    }
}
