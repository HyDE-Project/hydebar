//! Menu of the theme module: the desktop's look, and every way of changing
//! it.
//!
//! The menu is the only place the bar draws any of this. It states what the
//! desktop is on and lists what it could be on. The settings window used to
//! hold a page of its own and no longer does, so there is one list, one set
//! of chip states and one wait indicator rather than two that have to
//! be kept in step.

use std::collections::HashMap;

use hydebar_proto::{
    hyde_state::HydeState,
    theme_source::{Rgba, ThemeSwatch}
};
use iced::Element;

use super::{Message, Spinner};
use crate::components::{
    icons::Icons,
    page::{
        metrics::{
            chip_cell_width, chip_width, indicator_width, status_row_width, text_width,
            wrap_chips_into_rows
        },
        style, widgets,
        widgets::{
            ChipPaint, ThemeChip, grid, group, note, page, rows as row_stack, section,
            status_row, theme_chip
        }
    }
};

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
pub(super) fn view<'a>(
    state: &HydeState,
    swatches: &HashMap<String, ThemeSwatch>,
    screenshots: &HashMap<String, std::path::PathBuf>,
    switching: Option<&str>,
    catalogue: &[super::gallery::GalleryTheme],
    author: Option<&str>,
    installing: Option<&str>,
    condemned: Option<&str>,
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
        active_label(state, switching),
        busy_glyph,
        font_size
    ));

    let offered = offered_names(state, catalogue);
    let cell = shared_cell(state, &offered, list_layout, font_size, available_width);

    let mut window = page(font_size).push(active).push(section(
        THEMES,
        themes(
            state,
            swatches,
            screenshots,
            switching,
            condemned,
            updating,
            installing,
            catalogue,
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
            offer(
                state,
                catalogue,
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

/// Names the gallery offers that are not installed yet.
///
/// Matched through [`same_theme`], not equality: the catalogue spells
/// names with dashes where installed directories carry spaces, and a
/// literal comparison left every installed theme in the gallery, one
/// press away from downloading itself again.
pub(super) fn offered_names(
    state: &HydeState,
    catalogue: &[super::gallery::GalleryTheme]
) -> Vec<String> {
    catalogue
        .iter()
        .filter(|entry| {
            !state
                .themes
                .iter()
                .any(|installed| same_theme(installed, &entry.name))
        })
        .map(|entry| entry.name.clone())
        .collect()
}

/// One card width for both sections, whatever the layout.
///
/// The two grids used to size their cells from their own names and came
/// out unequal; sizing from every name at once is what makes an installed
/// card and an available one the same card.
fn shared_cell(
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
fn card_rows(
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

/// Whether two spellings name one theme.
///
/// Dashes and underscores stand in for spaces across the gallery, its
/// branches and the installed directories, and case drifts between them.
pub(super) fn same_theme(a: &str, b: &str) -> bool {
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

/// Renders the gallery as a grid of chips painted in announced colours.
///
/// One install at a time, and none beside a running switch: a chip being
/// installed carries the spinner, and every other chip waits unpressable
/// rather than starting a second writer over the same directories.
fn offer<'a>(
    state: &HydeState,
    catalogue: &[super::gallery::GalleryTheme],
    screenshots: &HashMap<String, std::path::PathBuf>,
    author: Option<&str>,
    switching: Option<&str>,
    installing: Option<&str>,
    spinner: Spinner,
    opacity: f32,
    font_size: f32,
    available_width: f32,
    cell: f32,
    list_layout: bool
) -> Element<'a, Message> {
    let names = offered_names(state, catalogue);
    let busy = switching.is_some() || installing.is_some();
    let mut block = grid(font_size);

    for indices in card_rows(&names, list_layout, font_size, available_width) {
        let mut row = group(font_size);

        for index in indices {
            let name = &names[index];
            let entry = catalogue.iter().find(|entry| &entry.name == name);

            let chip_state = if installing == Some(name.as_str()) {
                ThemeChip::Applying(spinner)
            } else if busy {
                ThemeChip::Blocked
            } else {
                ThemeChip::Idle
            };

            row = row.push(theme_chip(
                name.clone(),
                authored_badge(entry, author),
                Message::Install(name.clone()),
                chip_state,
                font_size,
                opacity,
                cell,
                entry.map(offer_paint),
                screenshots.get(&canonical(name)).cloned(),
                vec![(
                    Icons::Download.default_glyph(),
                    Message::Install(name.clone()),
                    !busy
                )],
                list_layout
            ));
        }

        block = block.push(row);
    }

    block.into()
}

/// The mark a card earns when its theme is the user's own work.
///
/// Ownership comes from the gallery index, and "the user" is whoever the
/// git identity of this machine names — the one signal that is already
/// there and already theirs.
fn authored_badge(
    entry: Option<&super::gallery::GalleryTheme>,
    author: Option<&str>
) -> Option<&'static str> {
    let owner = entry.map(|entry| entry.owner.as_str())?;
    let author = author?;

    owner
        .eq_ignore_ascii_case(author)
        .then(|| Icons::Authored.default_glyph())
}

#[cfg(test)]
mod badge_tests {
    use super::{super::gallery::GalleryTheme, authored_badge};

    fn entry(owner: &str) -> GalleryTheme {
        GalleryTheme {
            name:        "One Dark".to_owned(),
            link:        String::new(),
            owner:       owner.to_owned(),
            description: String::new(),
            colors:      [iced::Color::BLACK, iced::Color::WHITE]
        }
    }

    #[test]
    fn a_theme_of_the_local_author_is_marked() {
        let theme = entry("RAprogramm");

        assert!(authored_badge(Some(&theme), Some("raprogramm")).is_some());
    }

    #[test]
    fn foreign_and_unknown_work_stays_unmarked() {
        let theme = entry("someone-else");

        assert!(authored_badge(Some(&theme), Some("raprogramm")).is_none());
        assert!(authored_badge(Some(&theme), None).is_none());
        assert!(authored_badge(None, Some("raprogramm")).is_none());
    }
}

/// Paint for a gallery chip, from the two colours the index announces.
///
/// The index does not promise an order, so the darker of the two is taken
/// as the surface and the lighter as the ink — a chip the other way round
/// would be a swatch nobody can read.
fn offer_paint(entry: &super::gallery::GalleryTheme) -> ChipPaint {
    let luma = |color: iced::Color| 0.0722f32.mul_add(color.b, 0.7152f32.mul_add(color.g, 0.2126 * color.r));

    let [first, second] = entry.colors;
    let (surface, ink) = if luma(first) <= luma(second) {
        (first, second)
    } else {
        (second, first)
    };

    ChipPaint {
        background: surface,
        text:       ink,
        accent:     ink,
        palette:    entry.colors.to_vec()
    }
}

/// Renders the installed themes as a grid of chips.
///
/// The theme in force is drawn as picked, so the grid doubles as the answer
/// to "which one am I on" without the menu repeating the name twice.
///
/// While a switch runs the grid stops being a set of choices: the theme
/// being applied is the only one lit, and every other chip is dimmed
/// and carries no press at all. A second switch started on top of the
/// first would race it over the state file, the wallpaper cache and
/// every generated stylesheet, and the module refuses one anyway — a
/// grid that still looked pressable would only be hiding that refusal
/// until after the click.
fn themes<'a>(
    state: &HydeState,
    swatches: &HashMap<String, ThemeSwatch>,
    screenshots: &HashMap<String, std::path::PathBuf>,
    switching: Option<&str>,
    condemned: Option<&str>,
    updating: Option<&Option<String>>,
    installing: Option<&str>,
    catalogue: &[super::gallery::GalleryTheme],
    author: Option<&str>,
    spinner: Spinner,
    opacity: f32,
    font_size: f32,
    available_width: f32,
    cell: f32,
    list_layout: bool
) -> Element<'a, Message> {
    if state.themes.is_empty() {
        return note(NO_THEMES, font_size);
    }

    let locked = switching.is_some() || installing.is_some();
    let busy = locked || updating.is_some();
    let mut block =
        grid(font_size).push(update_all_row(busy, list_layout, font_size, opacity));

    for indices in card_rows(&state.themes, list_layout, font_size, available_width) {
        let mut row = group(font_size);

        for index in indices {
            let name = &state.themes[index];
            let doomed = condemned == Some(name.as_str());
            let fetching = matches!(updating, Some(Some(one)) if one == name);

            let (chip_state, press) = if doomed {
                (ThemeChip::Condemned, Message::Remove(name.clone()))
            } else if fetching {
                (ThemeChip::Applying(spinner), Message::Switch(name.clone()))
            } else if locked {
                (ThemeChip::Blocked, Message::Switch(name.clone()))
            } else {
                (
                    chip_state(state, switching, spinner, name),
                    Message::Switch(name.clone())
                )
            };

            let entry = catalogue.iter().find(|entry| same_theme(&entry.name, name));

            let paint = swatches
                .get(name)
                .map(chip_paint)
                .or_else(|| entry.map(offer_paint));

            let trash = if doomed {
                Message::Remove(name.clone())
            } else {
                Message::Condemn(name.clone())
            };

            let chip = theme_chip(
                name.clone(),
                authored_badge(entry, author),
                press,
                chip_state,
                font_size,
                opacity,
                cell,
                paint,
                screenshots.get(&canonical(name)).cloned(),
                vec![
                    (
                        Icons::Refresh.default_glyph(),
                        Message::Update(Some(name.clone())),
                        !busy
                    ),
                    (Icons::Trash.default_glyph(), trash, !busy || doomed),
                ],
                list_layout
            );

            row = row.push(
                iced::widget::mouse_area(chip).on_right_press(Message::Condemn(name.clone()))
            );
        }

        block = block.push(row);
    }

    block.into()
}

/// The row offering the one fetch that updates every installed theme.
fn update_all_row<'a>(
    busy: bool,
    list_layout: bool,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    use iced::widget::{Row, button};

    use crate::{
        components::{icons::icon_raw, scale},
        style::ghost_button_style
    };

    let control = style::control_size(font_size);
    let mut update = button(
        Row::new()
            .push(icon_raw(Icons::Refresh.default_glyph().to_owned()))
            .push(crate::components::text::text("Update all").size(control))
            .spacing(scale::icon_gap())
            .align_y(iced::Alignment::Center)
    )
    .padding(control * 0.25)
    .style(ghost_button_style(opacity));

    if !busy {
        update = update.on_press(Message::Update(None));
    }

    let layout_glyph = if list_layout {
        Icons::ViewGrid.default_glyph()
    } else {
        Icons::ViewList.default_glyph()
    };
    let layout = button(icon_raw(layout_glyph.to_owned()))
        .padding(control * 0.25)
        .style(ghost_button_style(opacity))
        .on_press(Message::ToggleLayout);

    Row::new()
        .push(update)
        .push(iced::widget::Space::new().width(iced::Length::Fill))
        .push(layout)
        .align_y(iced::Alignment::Center)
        .width(iced::Length::Fill)
        .into()
}

/// Restates a theme's swatch in the colours the renderer paints with.
fn chip_paint(swatch: &ThemeSwatch) -> ChipPaint {
    ChipPaint {
        background: colour(swatch.background),
        text:       colour(swatch.text),
        accent:     colour(swatch.accent),
        palette:    swatch.palette.map(colour).to_vec()
    }
}

/// One palette colour, as the renderer spells it.
const fn colour(rgba: Rgba) -> iced::Color {
    iced::Color::from_rgba8(rgba.r, rgba.g, rgba.b, rgba.a)
}

/// What the chip of `name` stands for, given what the desktop reports and
/// what the bar is waiting on.
///
/// The theme being applied wins over the theme in force, because `HyDE`
/// reports the new name in its state file long before the switch is
/// anywhere near done: a grid that only looked at the state file would
/// light the new chip within a moment of the press and then sit
/// completely still for the seconds that actually matter.
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
/// The theme in force always comes from `HyDE`'s own state file; a theme the
/// bar asked for is drawn beside it rather than in its place, because
/// until the switch has finished the desktop is still on the old one
/// and a menu that already named the new one would be reporting
/// something that may yet fail.
fn active_label(state: &HydeState, switching: Option<&str>) -> String {
    let active = state.theme.as_deref().unwrap_or(UNKNOWN);

    match switching {
        Some(pending) if !state.is_active(pending) => {
            format!("{active}{SWITCHING_TO}{pending}")
        }
        _ => active.to_owned()
    }
}

/// Rows the theme grid fills when laid out `available_width` wide.
fn theme_rows(state: &HydeState, font_size: f32, available_width: f32) -> f32 {
    theme_rows_in(state, false, font_size, available_width)
}

/// Rows the installed grid fills in the layout in force.
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
pub(super) fn desired_width(
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
pub(super) fn desired_height(
    state: &HydeState,
    offered: &[String],
    list_layout: bool,
    font_size: f32,
    available_width: f32
) -> f32 {
    let offered_rows = if offered.is_empty() {
        0.0
    } else {
        card_rows(offered, list_layout, font_size, available_width).len() as f32
    };
    let offered_sections = if offered.is_empty() { 0.0 } else { 1.0 };

    let chip_rows =
        theme_rows_in(state, list_layout, font_size, available_width) + offered_rows;
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

    /// HyDE writes the new name into its state file long before the switch
    /// is over, so the menu has to stop drawing an arrow that
    /// points at the theme it already reports.
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
    fn a_press_during_a_switch_is_heard_except_on_the_one_being_applied() {
        let state = state(&["Nord", "Mocha", "Latte"], Some("Nord"));
        let spinner = Spinner::default();

        for name in &state.themes {
            let pressable = chip_state(&state, Some("Mocha"), spinner, name).is_pressable();

            if name == "Mocha" {
                assert!(!pressable, "the running switch takes no second press");
            } else {
                assert!(pressable, "{name} must queue instead of going deaf");
            }
        }
    }

    /// The one case the state file cannot settle: HyDE records the new
    /// theme within a moment of the press and the switch runs for
    /// seconds afterwards, so a grid that trusted the file would go
    /// still exactly when the user is waiting hardest.
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
