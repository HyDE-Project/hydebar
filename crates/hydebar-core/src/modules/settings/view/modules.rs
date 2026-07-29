//! Module editor page of the settings window.
//!
//! The page is a small picture of the bar: three sections, the modules in the
//! order they are drawn, islands boxed together. At rest it says only what the
//! bar looks like. Picking a module opens a card that names it, says where it
//! sits, and offers its actions in labelled groups, so the page stays short
//! while every action is spelled out when it matters.
//!
//! Reordering is done with buttons rather than by dragging: a drag needs a
//! pointer held along a path, which a keyboard, a trackpad in accessibility
//! mode and a voice control cannot produce.

use hydebar_proto::config::Config;
use iced::Element;

use crate::{
    components::page::{
        metrics::{button_row_width, chip_width, wrap_chips_into_rows},
        style,
        widgets::{
            card, chip, choice_button, grid, group, labelled_row, note, outlined, page,
            rows as row_stack, section as titled
        }
    },
    modules::settings::{
        Message,
        layout::{Entry, LayoutEdit, Section, Slot, available}
    }
};

/// Label of the action moving a module one place earlier.
const MOVE_EARLIER: &str = "\u{2190} move left";
/// Label of the action moving a module one place later.
const MOVE_LATER: &str = "move right \u{2192}";
/// Label of the action moving a module to the section on the left.
const TO_LEFT: &str = "to Left";
/// Label of the action moving a module to the section on the right.
const TO_RIGHT: &str = "to Right";
/// Label of the action joining a module to the island beside it.
const MERGE: &str = "merge with the left";
/// Label of the action breaking a module out of its island.
const BREAK_OUT: &str = "break out";
/// Label of the action taking a module off the bar.
const REMOVE: &str = "take off the bar";

/// Title of the section picking which part of the bar is being edited.
const SECTION_PICKER: &str = "Section";

/// Title of the section showing what the bar carries today.
const ON_THE_BAR: &str = "On the bar";

/// Title of the section offering the modules that are not on the bar yet.
const CATALOGUE: &str = "Add a module";

/// Label of the row the reordering actions sit on.
const ORDER: &str = "Order";

/// Label of the row the moving and removing actions sit on.
const MOVE_IT: &str = "Move it";

/// Sections this page draws.
const SECTION_COUNT: f32 = 3.0;

/// Rows the section tab strip takes.
const TAB_ROWS: f32 = 1.0;

/// Rows the detail card takes: its heading and its two rows of actions.
const CARD_ROWS: f32 = 3.0;

/// Rows the catalogue takes before it wraps.
const CATALOGUE_ROWS: f32 = 1.0;

/// Groups the entries of a section into the islands they form.
fn islands(entries: &[Entry]) -> Vec<Vec<usize>> {
    let mut islands: Vec<Vec<usize>> = Vec::new();

    for (index, entry) in entries.iter().enumerate() {
        match (entry.joined, islands.last_mut()) {
            (true, Some(island)) => island.push(index),
            _ => islands.push(vec![index])
        }
    }

    islands
}

/// Island the module at `index` belongs to, counted from one.
fn island_of(entries: &[Entry], index: usize) -> usize {
    islands(entries)
        .into_iter()
        .position(|island| island.contains(&index))
        .map_or(1, |position| position + 1)
}

/// Renders the card describing the picked module and its actions.
///
/// The card is built from the same labelled rows as every other page, so a
/// module's actions line up with the steppers on the appearance tab rather than
/// forming a grid of their own.
fn detail<'a>(
    slot: Slot,
    entries: &[Entry],
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    let Some(entry) = entries.get(slot.index) else {
        return note("that module is gone", font_size);
    };

    let button = |label: &'static str, edit: LayoutEdit| {
        choice_button(label, Message::EditLayout(edit), false, font_size, opacity)
    };

    let heading = labelled_row(
        entry.module.as_str().to_owned(),
        note(
            format!(
                "{} section · island {} · position {} of {}",
                section_name(slot.section),
                island_of(entries, slot.index),
                slot.index + 1,
                entries.len()
            ),
            font_size
        ),
        font_size
    );

    let mut order = group(font_size);

    if slot.index > 0 {
        order = order.push(button(MOVE_EARLIER, LayoutEdit::MoveEarlier(slot)));
    }

    if slot.index + 1 < entries.len() {
        order = order.push(button(MOVE_LATER, LayoutEdit::MoveLater(slot)));
    }

    if slot.index == 0 && entries.len() == 1 {
        order = order.push(note("alone in this section", font_size));
    }

    let mut actions = group(font_size);

    if let Some(before) = slot.section.before() {
        actions = actions.push(button(
            section_button_label(before),
            LayoutEdit::MoveToPreviousSection(slot)
        ));
    }

    if let Some(after) = slot.section.after() {
        actions = actions.push(button(
            section_button_label(after),
            LayoutEdit::MoveToNextSection(slot)
        ));
    }

    actions = if slot.index == 0 {
        actions.push(note("first in the island", font_size))
    } else if entry.joined {
        actions.push(button(BREAK_OUT, LayoutEdit::ToggleJoin(slot)))
    } else {
        actions.push(button(MERGE, LayoutEdit::ToggleJoin(slot)))
    };

    actions = actions.push(button(REMOVE, LayoutEdit::Remove(slot)));

    card(
        row_stack(font_size)
            .push(heading)
            .push(labelled_row(ORDER, order.into(), font_size))
            .push(labelled_row(MOVE_IT, actions.into(), font_size))
            .into(),
        font_size,
        opacity
    )
}

/// Label of the button moving a module into `section`.
const fn section_button_label(section: Section) -> &'static str {
    match section {
        Section::Left => TO_LEFT,
        Section::Center => "to Center",
        Section::Right => TO_RIGHT
    }
}

/// Name of a section as the card spells it.
fn section_name(section: Section) -> &'static str {
    match section {
        Section::Left => "Left",
        Section::Center => "Center",
        Section::Right => "Right"
    }
}

/// Renders the modules that can still be added.
///
/// The chips wrap onto as many rows as the width allows, so a long catalogue
/// never runs past the edge of the window.
fn catalogue<'a>(
    config: &Config,
    section: Section,
    font_size: f32,
    opacity: f32,
    available_width: f32
) -> Element<'a, Message> {
    let custom = config
        .custom_modules
        .iter()
        .map(|module| module.name.clone())
        .collect::<Vec<_>>();

    let modules = available(&config.modules, &custom);

    if modules.is_empty() {
        return note("every module is already on the bar", font_size);
    }

    let gap = style::group_gap(font_size);
    let labels = modules
        .iter()
        .map(|module| module.as_str().to_owned())
        .collect::<Vec<_>>();

    let mut block = grid(font_size);

    for indices in wrap_chips_into_rows(&labels, available_width, font_size, gap) {
        let mut row = group(font_size);

        for index in indices {
            row = row.push(chip(
                labels[index].clone(),
                Message::EditLayout(LayoutEdit::Add {
                    section,
                    module: modules[index].clone()
                }),
                false,
                font_size,
                opacity
            ));
        }

        block = block.push(row);
    }

    block.into()
}

/// Renders the row of section tabs.
fn section_tabs<'a>(active: Section, font_size: f32, opacity: f32) -> Element<'a, Message> {
    let mut row = group(font_size);

    for section in Section::ALL {
        row = row.push(choice_button(
            section.label(),
            Message::SelectSection(section),
            section == active,
            font_size,
            opacity
        ));
    }

    row.into()
}

/// Renders the islands of one section, one island per row.
fn section_islands<'a>(
    section: Section,
    entries: &[Entry],
    selected: Option<Slot>,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    if entries.is_empty() {
        return note("this section is empty", font_size);
    }

    let mut column = row_stack(font_size);

    for (number, island) in islands(entries).into_iter().enumerate() {
        let mut chips = group(font_size);

        for index in island {
            let picked = selected
                == Some(Slot {
                    section,
                    index
                });

            chips = chips.push(chip(
                entries[index].module.as_str().to_owned(),
                Message::SelectSlot(if picked {
                    None
                } else {
                    Some(Slot {
                        section,
                        index
                    })
                }),
                picked,
                font_size,
                opacity
            ));
        }

        column = column.push(labelled_row(
            format!("island {}", number + 1),
            outlined(chips.into(), font_size, opacity),
            font_size
        ));
    }

    column.into()
}

/// Renders the module editor against the running `config`.
pub(super) fn view(
    config: &Config,
    opacity: f32,
    font_size: f32,
    section: Section,
    selected: Option<Slot>,
    available_width: f32
) -> Element<'_, Message> {
    let entries = section.entries(&config.modules);

    let mut bar = row_stack(font_size).push(section_islands(
        section, &entries, selected, font_size, opacity
    ));

    bar = match selected {
        Some(slot) if slot.section == section => {
            bar.push(detail(slot, &entries, font_size, opacity))
        }
        _ => bar.push(note("pick a module to move, group or remove it", font_size))
    };

    page(font_size)
        .push(titled(
            SECTION_PICKER,
            section_tabs(section, font_size, opacity),
            font_size
        ))
        .push(titled(ON_THE_BAR, bar.into(), font_size))
        .push(titled(
            CATALOGUE,
            catalogue(config, section, font_size, opacity, available_width),
            font_size
        ))
        .into()
}

/// Labels of the actions one card row can offer, so the width is measured
/// against the very strings that are drawn.
const ACTION_LABELS: [&str; 4] = [TO_LEFT, TO_RIGHT, MERGE, REMOVE];

/// Rows this page draws for `section`, its section headings counted in.
#[must_use]
pub(super) fn rows(config: &Config, section: Section) -> f32 {
    let entries = section.entries(&config.modules);

    SECTION_COUNT * style::SECTION_TITLE_ROWS
        + TAB_ROWS
        + islands(&entries).len().max(1) as f32
        + CARD_ROWS
        + CATALOGUE_ROWS
}

/// Longest line of this page, which is how wide the window has to be.
///
/// Only the lines that are actually drawn are measured: the section tabs, the
/// widest island of the section on show, and the widest the action card can
/// become. The catalogue is left out on purpose, since it wraps into whatever
/// width the rest settles on.
#[must_use]
pub(super) fn desired_width(config: &Config, font_size: f32, section: Section) -> f32 {
    let control = style::control_size(font_size);
    let gap = style::group_gap(font_size);
    let entries = section.entries(&config.modules);

    let tabs = button_row_width(Section::ALL.into_iter().map(Section::label), control, gap);

    let widest_island = islands(&entries)
        .into_iter()
        .map(|island| {
            let count = island.len() as f32;
            let chips: f32 = island
                .into_iter()
                .map(|index| chip_width(entries[index].module.as_str(), control))
                .sum();

            labelled_row_width(font_size)
                + chips
                + gap * (count - 1.0).max(0.0)
                + style::card_overhead(font_size)
        })
        .fold(0.0_f32, f32::max);

    let card = labelled_row_width(font_size)
        + button_row_width(ACTION_LABELS.into_iter(), control, gap)
        + style::card_overhead(font_size);

    tabs.max(widest_island).max(card)
}

/// Room a labelled row spends before its controls start.
///
/// The same label column every other page reserves, so an island lines up with
/// a stepper on the appearance tab.
fn labelled_row_width(font_size: f32) -> f32 {
    style::label_width(font_size) + style::row_gap(font_size)
}

/// Height this page needs for `section`.
#[must_use]
pub(super) fn desired_height(config: &Config, font_size: f32, section: Section) -> f32 {
    style::page_height(rows(config, section), font_size)
}

#[cfg(test)]
mod tests {
    use hydebar_proto::config::{ModuleDef, ModuleName, Modules};

    use super::{super::metrics::text_width, *};

    fn entries(left: Vec<ModuleDef>) -> Vec<Entry> {
        Section::Left.entries(&Modules {
            left,
            center: Vec::new(),
            right: Vec::new()
        })
    }

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
    fn neighbouring_joined_modules_form_one_island() {
        let section = entries(vec![
            ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Tray]),
            ModuleDef::Single(ModuleName::Battery),
        ]);

        assert_eq!(islands(&section), vec![vec![0, 1], vec![2]]);
    }

    #[test]
    fn a_section_of_singles_is_a_row_of_islands() {
        let section = entries(vec![
            ModuleDef::Single(ModuleName::Clock),
            ModuleDef::Single(ModuleName::Tray),
        ]);

        assert_eq!(islands(&section), vec![vec![0], vec![1]]);
    }

    #[test]
    fn an_empty_section_has_no_islands() {
        assert!(islands(&[]).is_empty());
    }

    #[test]
    fn a_module_knows_which_island_it_sits_in() {
        let section = entries(vec![
            ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Tray]),
            ModuleDef::Single(ModuleName::Battery),
        ]);

        assert_eq!(island_of(&section, 0), 1);
        assert_eq!(island_of(&section, 1), 1);
        assert_eq!(island_of(&section, 2), 2);
    }

    #[test]
    fn a_missing_module_is_reported_as_the_first_island() {
        assert_eq!(island_of(&[], 3), 1);
    }

    #[test]
    fn an_empty_section_still_reserves_a_row_for_its_notice() {
        let empty = config(Vec::new());

        assert_eq!(
            rows(&empty, Section::Left),
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
            desired_height(&many, 16.0, Section::Left) > desired_height(&few, 16.0, Section::Left)
        );
    }

    #[test]
    fn an_island_row_reserves_the_shared_label_column() {
        let font_size = 16.0;
        let modules = config(vec![ModuleDef::Single(ModuleName::Clock)]);

        assert!(desired_width(&modules, font_size, Section::Left) > style::label_width(font_size));
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

        assert_eq!(
            desired_height(&modules, font_size, Section::Left),
            style::page_height(rows(&modules, Section::Left), font_size)
        );
    }
}
