//! HyDE page of the settings window.
//!
//! The page is about the desktop rather than about the bar, so nothing on it is
//! written into the bar's configuration file: pressing a theme asks
//! `hyde-shell` to switch, and the desktop — the bar included — follows. What
//! the page shows is read back from HyDE's own state, so it reports the desktop
//! as it is even when the change came from a keybinding rather than from here.
//!
//! Facts HyDE owns but does not take an instruction for, such as whether the
//! colours are pulled from the wallpaper, are shown as plain text: a control
//! that cannot act is worse than a line that simply states the truth.

use hydebar_proto::hyde_state::HydeState;
use iced::{
    Element, Length,
    widget::{Column, Row, text}
};

use super::{
    metrics::{ROW_HEIGHT_EM, button_width, text_width, wrap_into_rows},
    widgets::{ROW_GAP_EM, action_group, caption, chip, choice_button, status_row}
};
use crate::modules::settings::Message;

/// Gap between the rows of the page, in multiples of the text size.
const PAGE_GAP_EM: f32 = 1.0;

/// Size a theme button is drawn at, relative to the page text size.
///
/// The themes are a long list, so they are drawn a little smaller than the rest
/// of the page to keep the window from growing taller than the screen.
const THEME_FONT_SCALE: f32 = 0.85;

/// Theme buttons a row is sized to hold.
///
/// The page has to name a width before it knows how the buttons wrap, and this
/// is the number that keeps a typical HyDE install to a handful of rows without
/// making the window wider than the pages beside it.
const THEMES_PER_ROW: f32 = 3.0;

/// Label of the button asking HyDE for another wallpaper.
const NEXT_WALLPAPER: &str = "next wallpaper";

/// Shown in place of the theme name while HyDE has recorded none.
const UNKNOWN: &str = "unknown";

/// Shown in place of the theme list while none are installed.
const NO_THEMES: &str = "no HyDE themes found on this machine";

/// Renders the HyDE page against the desktop state last read from disk.
///
/// `available_width` is how wide the page may draw, so the theme buttons wrap
/// inside the window instead of running past its edge.
pub(super) fn view<'a>(
    state: &HydeState,
    opacity: f32,
    font_size: f32,
    available_width: f32
) -> Element<'a, Message> {
    let theme_font = font_size * THEME_FONT_SCALE;
    let gap = ROW_GAP_EM * font_size;

    Column::new()
        .push(status_row(
            "Active theme",
            state.theme.clone().unwrap_or_else(|| UNKNOWN.to_owned()),
            font_size
        ))
        .push(themes(state, opacity, font_size, available_width))
        .push(status_row(
            "Wallpaper colours",
            switch_label(state.wallpaper_colors).to_owned(),
            font_size
        ))
        .push(status_row(
            "Shader",
            state.shader.clone().unwrap_or_else(|| UNKNOWN.to_owned()),
            font_size
        ))
        .push(action_group(
            "Wallpaper",
            vec![choice_button(
                NEXT_WALLPAPER,
                Message::NextHydeWallpaper,
                false,
                theme_font,
                opacity
            )],
            font_size
        ))
        .width(Length::Fill)
        .spacing(gap)
        .into()
}

/// Renders the installed themes as a grid of buttons.
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
        return Column::new()
            .push(caption("Themes", font_size))
            .push(text(NO_THEMES).size(font_size * THEME_FONT_SCALE))
            .spacing(ROW_GAP_EM * font_size * 0.35)
            .into();
    }

    let theme_font = font_size * THEME_FONT_SCALE;
    let gap = ROW_GAP_EM * font_size * 0.5;
    let mut grid = Column::new()
        .push(caption("Themes", font_size))
        .spacing(gap)
        .width(Length::Fill);

    for indices in wrap_into_rows(&state.themes, available_width, theme_font, gap) {
        let mut row = Row::new().spacing(gap).width(Length::Fill);

        for index in indices {
            let name = &state.themes[index];

            row = row.push(chip(
                name.clone(),
                Message::SwitchHydeTheme(name.clone()),
                state.is_active(name),
                theme_font,
                opacity
            ));
        }

        grid = grid.push(row);
    }

    grid.into()
}

/// Names the state of a switch the page reports but does not operate.
fn switch_label(enabled: bool) -> &'static str {
    if enabled { "On" } else { "Off" }
}

/// Rows the theme grid fills when laid out `available_width` wide.
fn theme_rows(state: &HydeState, font_size: f32, available_width: f32) -> f32 {
    if state.themes.is_empty() {
        return 1.0;
    }

    let theme_font = font_size * THEME_FONT_SCALE;
    let gap = ROW_GAP_EM * font_size * 0.5;

    wrap_into_rows(&state.themes, available_width, theme_font, gap).len() as f32
}

/// Longest line of this page, which is how wide the window has to be.
///
/// The theme grid is measured as a row of the widest themes rather than as the
/// whole list: the grid wraps into whatever width the rest of the page settles
/// on, and sizing the window to hold every theme side by side would make it far
/// wider than the screen.
#[must_use]
pub(super) fn desired_width(state: &HydeState, font_size: f32) -> f32 {
    let gap = ROW_GAP_EM * font_size;
    let theme_font = font_size * THEME_FONT_SCALE;

    let statuses = [
        ("Active theme", state.theme.as_deref().unwrap_or(UNKNOWN)),
        ("Wallpaper colours", switch_label(state.wallpaper_colors)),
        ("Shader", state.shader.as_deref().unwrap_or(UNKNOWN))
    ]
    .into_iter()
    .map(|(label, value)| text_width(label, font_size) + gap + text_width(value, font_size))
    .fold(0.0_f32, f32::max);

    let grid = if state.themes.is_empty() {
        text_width(NO_THEMES, theme_font)
    } else {
        let widest = state
            .themes
            .iter()
            .map(|name| button_width(name, theme_font))
            .fold(0.0_f32, f32::max);

        widest * THEMES_PER_ROW + gap * (THEMES_PER_ROW - 1.0)
    };

    let wallpaper = button_width(NEXT_WALLPAPER, theme_font);

    statuses.max(grid).max(wallpaper)
}

/// Height this page needs when drawn `available_width` wide.
#[must_use]
pub(super) fn desired_height(state: &HydeState, font_size: f32, available_width: f32) -> f32 {
    let row = ROW_HEIGHT_EM * font_size + PAGE_GAP_EM * font_size;

    let statuses = row * 3.0;
    let grid = row * (theme_rows(state, font_size, available_width) + 1.0);
    let wallpaper = row * 2.0;

    statuses + grid + wallpaper
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
        assert!(desired_width(&HydeState::default(), 16.0) > 0.0);
    }

    #[test]
    fn a_page_without_themes_reserves_one_row_for_the_notice() {
        assert_eq!(theme_rows(&HydeState::default(), 16.0, 400.0), 1.0);
    }

    #[test]
    fn a_long_theme_name_widens_the_page() {
        let short = state(&["Nord"], Some("Nord"));
        let long = state(&["An Extremely Long Theme Name"], Some("Nord"));

        assert!(desired_width(&long, 16.0) > desired_width(&short, 16.0));
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
        let width = desired_width(&few, 16.0);

        assert!(desired_height(&many, 16.0, width) > desired_height(&few, 16.0, width));
    }

    #[test]
    fn every_theme_lands_in_exactly_one_row() {
        let themes = state(
            &["Nord", "Mocha", "Latte", "Decay Green", "Edge Runner"],
            None
        );
        let rows = wrap_into_rows(&themes.themes, 200.0, 16.0 * THEME_FONT_SCALE, 8.0);

        assert_eq!(
            rows.iter().map(Vec::len).sum::<usize>(),
            themes.themes.len()
        );
    }
}
