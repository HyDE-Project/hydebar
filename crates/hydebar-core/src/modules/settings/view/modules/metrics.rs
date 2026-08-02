//! Measurements of the room the module editor asks for.

use super::{detail::ACTION_LABELS, islands};
use crate::{
    components::page::{
        metrics::{button_row_width, chip_width},
        style
    },
    modules::settings::layout::{Entry, Section}
};

/// Sections this page draws.
const SECTION_COUNT: f32 = 3.0;

/// Rows the section tab strip takes.
const TAB_ROWS: f32 = 1.0;

/// Rows the detail card takes: its heading and its two rows of actions.
const CARD_ROWS: f32 = 3.0;

/// Rows the catalogue takes before it wraps.
const CATALOGUE_ROWS: f32 = 1.0;

/// Rows this page draws for `section`, its section headings counted in.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "island counts are small, fit f32 exactly"
)]
pub fn rows(entries: &[Entry]) -> f32 {
    SECTION_COUNT.mul_add(style::SECTION_TITLE_ROWS, TAB_ROWS)
        + islands(entries).len().max(1) as f32
        + CARD_ROWS
        + CATALOGUE_ROWS
}

/// Longest line of this page, which is how wide the window has to be.
///
/// Only the lines that are actually drawn are measured: the section
/// tabs, the widest island of the section on show, and the
/// widest the action card can become. The catalogue is left out
/// on purpose, since it wraps into whatever width the rest
/// settles on.
#[must_use]
pub fn desired_width(font_size: f32, entries: &[Entry]) -> f32 {
    let control = style::control_size(font_size);
    let gap = style::group_gap(font_size);

    let tabs = button_row_width(Section::ALL.into_iter().map(Section::label), control, gap);

    #[expect(
        clippy::cast_precision_loss,
        reason = "island sizes are small, fit f32 exactly"
    )]
    let widest_island = islands(entries)
        .into_iter()
        .map(|island| {
            let count = island.len() as f32;
            let chips: f32 = island
                .into_iter()
                .map(|index| chip_width(entries[index].module.as_str(), control))
                .sum();

            gap.mul_add(
                (count - 1.0).max(0.0),
                labelled_row_width(font_size) + chips
            ) + style::card_overhead(font_size)
        })
        .fold(0.0_f32, f32::max);

    let card = labelled_row_width(font_size)
        + button_row_width(ACTION_LABELS, control, gap)
        + style::card_overhead(font_size);

    tabs.max(widest_island).max(card)
}

/// Room a labelled row spends before its controls start.
///
/// The same label column every other page reserves, so an island lines
/// up with a stepper on the appearance tab.
fn labelled_row_width(font_size: f32) -> f32 {
    style::label_width(font_size) + style::row_gap(font_size)
}

/// Height this page needs for the entries on show.
#[must_use]
pub fn desired_height(font_size: f32, entries: &[Entry]) -> f32 {
    style::page_height(rows(entries), font_size)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use hydebar_proto::config::{Config, ModuleDef, ModuleName, Modules};

    use super::{
        super::detail::{MOVE_IT, ORDER},
        *
    };
    use crate::components::page::metrics::text_width;

    fn config(left: Vec<ModuleDef>) -> Config {
        Config {
            modules: Modules {
                left,
                center: Vec::new(),
                right: Vec::new()
            },
            ..Config::default()
        }
    }

    #[test]
    fn an_empty_section_still_reserves_a_row_for_its_notice() {
        let empty = config(Vec::new());

        assert_eq!(
            rows(&Section::Left.entries(&empty.modules)),
            SECTION_COUNT + TAB_ROWS + 1.0 + CARD_ROWS + CATALOGUE_ROWS
        );
    }

    #[test]
    fn more_islands_make_the_page_taller() {
        let few = config(vec![ModuleDef::Single(ModuleName::Clock)]);
        let many = config(vec![
            ModuleDef::Single(ModuleName::Clock),
            ModuleDef::Single(ModuleName::Tray),
            ModuleDef::Single(ModuleName::Battery),
        ]);

        assert!(
            desired_height(16.0, &Section::Left.entries(&many.modules))
                > desired_height(16.0, &Section::Left.entries(&few.modules))
        );
    }

    #[test]
    fn an_island_row_reserves_the_shared_label_column() {
        let font_size = 16.0;
        let modules = config(vec![ModuleDef::Single(ModuleName::Clock)]);

        assert!(
            desired_width(font_size, &Section::Left.entries(&modules.modules))
                > style::label_width(font_size)
        );
    }

    #[test]
    fn every_row_label_fits_the_shared_label_column() {
        let font_size = 16.0;

        for label in ["island 00", ORDER, MOVE_IT] {
            assert!(
                text_width(label, font_size) <= style::label_width(font_size),
                "{label} overflows the label column"
            );
        }
    }

    #[test]
    fn the_page_height_follows_the_shared_row_pitch() {
        let font_size = 16.0;
        let modules = config(vec![ModuleDef::Single(ModuleName::Clock)]);

        let entries = Section::Left.entries(&modules.modules);
        assert_eq!(
            desired_height(font_size, &entries),
            style::page_height(rows(&entries), font_size)
        );
    }
}
