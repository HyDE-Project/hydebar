//! The installed section: cards for the themes already on this machine.

use std::collections::HashMap;

use hydebar_proto::{
    hyde_state::HydeState,
    theme_source::{Rgba, ThemeSwatch}
};
use iced::Element;

use super::{
    NO_THEMES, canonical,
    gallery_cards::{authored_badge, offer_paint},
    sizing::card_rows,
    toolbar::update_all_row
};
use crate::{
    components::{
        icons::Icons,
        page::widgets::{ChipPaint, ThemeChip, grid, group, note, theme_chip}
    },
    modules::themes::{Message, Spinner, gallery::GalleryTheme}
};

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
#[expect(
    clippy::too_many_arguments,
    reason = "view helper mirrors the fields of the state it renders"
)]
pub(super) fn themes<'a>(
    state: &HydeState,
    swatches: &HashMap<String, ThemeSwatch>,
    screenshots: &HashMap<String, std::path::PathBuf>,
    switching: Option<&str>,
    updating: Option<&Option<String>>,
    installing: Option<&str>,
    catalogue: &[GalleryTheme],
    catalogue_index: &HashMap<String, usize>,
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
    let mut block = grid(font_size).push(update_all_row(busy, list_layout, font_size, opacity));

    for indices in card_rows(&state.themes, list_layout, font_size, available_width) {
        let mut row = group(font_size);

        for index in indices {
            let name = &state.themes[index];
            let fetching = matches!(updating, Some(Some(one)) if one == name);

            let chip_look = if fetching {
                ThemeChip::Applying(spinner)
            } else if locked {
                ThemeChip::Blocked
            } else {
                chip_state(state, switching, spinner, name)
            };
            let press = Message::Switch(name.clone());

            let entry = catalogue_index
                .get(&canonical(name))
                .map(|&index| &catalogue[index]);

            let paint = swatches
                .get(name)
                .map(chip_paint)
                .or_else(|| entry.map(offer_paint));

            let trash = Message::Remove(name.clone());

            let chip = theme_chip(
                name.clone(),
                authored_badge(entry, author),
                press,
                chip_look,
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
                    (Icons::Trash.default_glyph(), trash, true),
                ],
                list_layout
            );

            row = row.push(chip);
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
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

    /// The one case the state file cannot settle: `HyDE` records the new
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
}
