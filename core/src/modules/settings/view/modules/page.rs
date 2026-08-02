//! Assembly of the module editor page from its sections.

use hydebar_proto::config::Config;
use iced::Element;

use super::{catalogue::catalogue, detail::detail, islands};
use crate::{
    components::page::widgets::{
        chip, choice_button, group, labelled_row, note, outlined, page, rows as row_stack,
        section as titled
    },
    modules::settings::{
        Message,
        layout::{Entry, Section, Slot}
    }
};

/// Title of the section picking which part of the bar is being edited.
const SECTION_PICKER: &str = "Section";

/// Title of the section showing what the bar carries today.
const ON_THE_BAR: &str = "On the bar";

/// Title of the section offering the modules that are not on the bar
/// yet.
const CATALOGUE: &str = "Add a module";

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
                opacity,
                None
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
pub fn view<'a>(
    config: &'a Config,
    opacity: f32,
    font_size: f32,
    section: Section,
    selected: Option<Slot>,
    available_width: f32,
    entries: &[Entry]
) -> Element<'a, Message> {
    let mut bar = row_stack(font_size).push(section_islands(
        section, entries, selected, font_size, opacity
    ));

    bar = match selected {
        Some(slot) if slot.section == section => {
            bar.push(detail(slot, entries, font_size, opacity))
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
