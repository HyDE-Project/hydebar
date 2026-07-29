//! Module editor page of the settings window.
//!
//! The page is a small picture of the bar: three rows, the modules in the order
//! they are drawn, islands boxed together. At rest it says only what the bar
//! looks like. Picking a module opens a card that names it, says where it sits,
//! and offers its actions in labelled groups, so the page stays short while
//! every action is spelled out when it matters.
//!
//! Reordering is done with buttons rather than by dragging: a drag needs a
//! pointer held along a path, which a keyboard, a trackpad in accessibility
//! mode and a voice control cannot produce.

use hydebar_proto::config::Config;
use iced::{
    Alignment, Border, Element, Length, Theme,
    widget::{Column, Row, container, text}
};

use super::{
    metrics::{ROW_HEIGHT_EM, button_width, wrap_into_rows},
    widgets::{ROW_GAP_EM, action_group, caption, card, chip, choice_button}
};
use crate::modules::settings::{
    Message,
    layout::{Entry, LayoutEdit, Section, Slot, available}
};

/// Gap between the rows of the page, in multiples of the text size.
const PAGE_GAP_EM: f32 = 0.8;

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

/// Padding inside the box drawn around an island, in multiples of the text
/// size.
const ISLAND_PADDING_EM: f32 = 0.3;

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
fn detail<'a>(
    slot: Slot,
    entries: &[Entry],
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    let Some(entry) = entries.get(slot.index) else {
        return text("that module is gone").size(font_size * 0.85).into();
    };

    let gap = ROW_GAP_EM * font_size;
    let button = |label: &'static str, edit: LayoutEdit| {
        choice_button(
            label,
            Message::EditLayout(edit),
            false,
            font_size * 0.8,
            opacity
        )
    };

    let heading = Row::new()
        .push(text(entry.module.as_str().to_owned()).size(font_size))
        .push(
            text(format!(
                "{} section · island {} · position {} of {}",
                section_name(slot.section),
                island_of(entries, slot.index),
                slot.index + 1,
                entries.len()
            ))
            .size(font_size * 0.75)
        )
        .spacing(gap * 0.6)
        .align_y(Alignment::Center);

    let mut groups = Row::new().spacing(gap).align_y(Alignment::Start);

    let mut order = Vec::new();

    if slot.index > 0 {
        order.push(button(MOVE_EARLIER, LayoutEdit::MoveEarlier(slot)));
    }

    if slot.index + 1 < entries.len() {
        order.push(button(MOVE_LATER, LayoutEdit::MoveLater(slot)));
    }

    if order.is_empty() {
        order.push(text("alone in this section").size(font_size * 0.75).into());
    }

    groups = groups.push(action_group("Order", order, font_size));

    let mut sections = Vec::new();

    if let Some(before) = slot.section.before() {
        sections.push(button(
            section_button_label(before),
            LayoutEdit::MoveToPreviousSection(slot)
        ));
    }

    if let Some(after) = slot.section.after() {
        sections.push(button(
            section_button_label(after),
            LayoutEdit::MoveToNextSection(slot)
        ));
    }

    groups = groups.push(action_group("Move to section", sections, font_size));

    let island = if slot.index == 0 {
        vec![text("first in the section").size(font_size * 0.75).into()]
    } else if entry.joined {
        vec![button(BREAK_OUT, LayoutEdit::ToggleJoin(slot))]
    } else {
        vec![button(MERGE, LayoutEdit::ToggleJoin(slot))]
    };

    groups = groups.push(action_group("Island", island, font_size));
    groups = groups.push(action_group(
        "Remove",
        vec![button(REMOVE, LayoutEdit::Remove(slot))],
        font_size
    ));

    card(
        Column::new()
            .push(heading)
            .push(groups)
            .spacing(gap * 0.7)
            .width(Length::Fill)
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
        return caption("Every module is already on the bar", font_size);
    }

    let gap = ROW_GAP_EM * font_size * 0.3;
    let chip_font = font_size * 0.8;
    let labels = modules
        .iter()
        .map(|module| module.as_str().to_owned())
        .collect::<Vec<_>>();

    let mut rows = Column::new().spacing(gap).width(Length::Fill);

    for indices in wrap_into_rows(&labels, available_width, chip_font, gap) {
        let mut row = Row::new().spacing(gap).width(Length::Fill);

        for index in indices {
            row = row.push(chip(
                labels[index].clone(),
                Message::EditLayout(LayoutEdit::Add {
                    section,
                    module: modules[index].clone()
                }),
                false,
                chip_font,
                opacity
            ));
        }

        rows = rows.push(row);
    }

    Column::new()
        .push(caption(
            match section {
                Section::Left => "Add to the Left section",
                Section::Center => "Add to the Center section",
                Section::Right => "Add to the Right section"
            },
            font_size
        ))
        .push(rows)
        .spacing(gap)
        .width(Length::Fill)
        .into()
}

/// Renders the row of section tabs.
fn section_tabs<'a>(active: Section, font_size: f32, opacity: f32) -> Element<'a, Message> {
    let mut row = Row::new().spacing(ROW_GAP_EM * font_size * 0.5);

    for section in Section::ALL {
        row = row.push(choice_button(
            section.label(),
            Message::SelectSection(section),
            section == active,
            font_size * 0.85,
            opacity
        ));
    }

    row.into()
}

/// Renders the islands of one section, one island per line.
fn section_islands<'a>(
    section: Section,
    entries: &[Entry],
    selected: Option<Slot>,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    let gap = ROW_GAP_EM * font_size;
    let mut column = Column::new().spacing(gap * 0.5).width(Length::Fill);

    if entries.is_empty() {
        return caption("This section is empty", font_size);
    }

    for (number, island) in islands(entries).into_iter().enumerate() {
        let mut chips = Row::new().spacing(gap * 0.3).align_y(Alignment::Center);

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
                font_size * 0.85,
                opacity
            ));
        }

        column = column.push(
            Row::new()
                .push(
                    text(format!("island {}", number + 1))
                        .size(font_size * 0.75)
                        .width(Length::Fixed(button_width("island 00", font_size * 0.75)))
                )
                .push(
                    container(chips)
                        .padding(ISLAND_PADDING_EM * font_size)
                        .style(move |theme: &Theme| container::Style {
                            border: Border {
                                width:  1.0,
                                radius: (font_size * 0.55).into(),
                                color:  theme
                                    .extended_palette()
                                    .secondary
                                    .strong
                                    .color
                                    .scale_alpha(opacity)
                            },
                            ..container::Style::default()
                        })
                )
                .spacing(gap * 0.5)
                .align_y(Alignment::Center)
                .width(Length::Fill)
        );
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

    let mut page = Column::new()
        .push(section_tabs(section, font_size, opacity))
        .push(section_islands(
            section, &entries, selected, font_size, opacity
        ))
        .spacing(PAGE_GAP_EM * font_size)
        .width(Length::Fill);

    page = match selected {
        Some(slot) if slot.section == section => {
            page.push(detail(slot, &entries, font_size, opacity))
        }
        _ => page.push(caption(
            "Pick a module to move, group or remove it",
            font_size
        ))
    };

    page.push(catalogue(
        config,
        section,
        font_size,
        opacity,
        available_width
    ))
    .into()
}

/// Labels of the actions the card can offer, so the width is measured against
/// the very strings that are drawn.
const ACTION_LABELS: [&str; 6] = [MOVE_EARLIER, MOVE_LATER, TO_LEFT, TO_RIGHT, MERGE, REMOVE];

/// Longest line of this page, which is how wide the window has to be.
///
/// Only the lines that are actually drawn are measured: the section tabs, the
/// widest island of the section on show, and the widest the action card can
/// become. The catalogue is left out on purpose, since it wraps into whatever
/// width the rest settles on.
#[must_use]
pub(super) fn desired_width(config: &Config, font_size: f32, section: Section) -> f32 {
    let gap = ROW_GAP_EM * font_size;
    let entries = section.entries(&config.modules);

    let tabs = Section::ALL
        .into_iter()
        .map(|section| button_width(section.label(), font_size * 0.85) + gap * 0.5)
        .sum::<f32>();

    let widest_island = islands(&entries)
        .into_iter()
        .map(|island| {
            let chips: f32 = island
                .into_iter()
                .map(|index| {
                    button_width(entries[index].module.as_str(), font_size * 0.85) + gap * 0.3
                })
                .sum();

            button_width("island 00", font_size * 0.75) + gap * 0.5 + chips
        })
        .fold(0.0_f32, f32::max);

    let card = ACTION_LABELS
        .into_iter()
        .map(|label| button_width(label, font_size * 0.8) + gap)
        .sum::<f32>();

    tabs.max(widest_island).max(card)
}

#[cfg(test)]
mod tests {
    use hydebar_proto::config::{ModuleDef, ModuleName, Modules};

    use super::*;

    fn entries(left: Vec<ModuleDef>) -> Vec<Entry> {
        Section::Left.entries(&Modules {
            left,
            center: Vec::new(),
            right: Vec::new()
        })
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
}

/// Height this page needs for `section`.
pub(super) fn desired_height(config: &Config, font_size: f32, section: Section) -> f32 {
    let row = ROW_HEIGHT_EM * font_size + PAGE_GAP_EM * font_size;
    let entries = section.entries(&config.modules);

    let tabs = row;
    let islands = islands(&entries).len().max(1) as f32 * row;
    let card = row * 3.0;
    let catalogue = row * 2.0;

    tabs + islands + card + catalogue
}
