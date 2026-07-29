//! Module editor page of the settings window.
//!
//! Entries are rearranged with buttons rather than by dragging: a keyboard, a
//! trackpad and a voice control all reach a button, while a drag needs a
//! pointer held down along a path.

use hydebar_proto::config::{Config, ModuleDef, ModuleName};
use iced::{
    Alignment, Border, Element, Length, Theme,
    widget::{Column, Row, container, text}
};

use super::widgets::{ROW_GAP_EM, choice_button};
use crate::modules::settings::{
    Message,
    layout::{LayoutEdit, Section, available}
};

/// Gap between the rows of a section, in multiples of the text size.
const SECTION_GAP_EM: f32 = 0.5;
/// Gap between the sections, in multiples of the text size.
const PAGE_GAP_EM: f32 = 1.2;
/// Padding inside the box drawn around an island, in multiples of the text
/// size.
const ISLAND_PADDING_EM: f32 = 0.4;

/// Names the modules an entry holds, joined for a single line.
fn entry_label(entry: &ModuleDef) -> String {
    match entry {
        ModuleDef::Single(name) => name.as_str().to_owned(),
        ModuleDef::Group(group) => group
            .iter()
            .map(ModuleName::as_str)
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

/// Renders one entry with the buttons acting on it.
fn entry_row<'a>(
    section: Section,
    index: usize,
    entry: &ModuleDef,
    last: bool,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    let grouped = matches!(entry, ModuleDef::Group(_));

    let mut buttons = Row::new().spacing(ROW_GAP_EM * font_size * 0.5);

    buttons = buttons.push(choice_button(
        "↑",
        Message::EditLayout(LayoutEdit::MoveUp {
            section,
            index
        }),
        false,
        font_size,
        opacity
    ));
    buttons = buttons.push(choice_button(
        "↓",
        Message::EditLayout(LayoutEdit::MoveDown {
            section,
            index
        }),
        false,
        font_size,
        opacity
    ));
    buttons = buttons.push(choice_button(
        "→",
        Message::EditLayout(LayoutEdit::MoveToNextSection {
            section,
            index
        }),
        false,
        font_size,
        opacity
    ));

    if grouped {
        buttons = buttons.push(choice_button(
            "split",
            Message::EditLayout(LayoutEdit::Ungroup {
                section,
                index
            }),
            false,
            font_size,
            opacity
        ));
    } else if !last {
        buttons = buttons.push(choice_button(
            "join",
            Message::EditLayout(LayoutEdit::GroupWithNext {
                section,
                index
            }),
            false,
            font_size,
            opacity
        ));
    }

    buttons = buttons.push(choice_button(
        "✕",
        Message::EditLayout(LayoutEdit::Remove {
            section,
            index
        }),
        false,
        font_size,
        opacity
    ));

    let row = Row::new()
        .push(text(entry_label(entry)).size(font_size).width(Length::Fill))
        .push(buttons)
        .align_y(Alignment::Center)
        .spacing(ROW_GAP_EM * font_size)
        .width(Length::Fill);

    if grouped {
        container(row)
            .padding(ISLAND_PADDING_EM * font_size)
            .style(move |theme: &Theme| container::Style {
                border: Border {
                    width:  1.0,
                    radius: (font_size * 0.4).into(),
                    color:  theme
                        .extended_palette()
                        .secondary
                        .strong
                        .color
                        .scale_alpha(opacity)
                },
                ..container::Style::default()
            })
            .width(Length::Fill)
            .into()
    } else {
        container(row).width(Length::Fill).into()
    }
}

/// Renders one column of the editor.
fn section_column<'a>(
    section: Section,
    entries: &[ModuleDef],
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    let mut column = Column::new()
        .push(text(section.label()).size(font_size))
        .spacing(SECTION_GAP_EM * font_size)
        .width(Length::Fill);

    if entries.is_empty() {
        column = column.push(text("empty").size(font_size * 0.9));
    }

    for (index, entry) in entries.iter().enumerate() {
        column = column.push(entry_row(
            section,
            index,
            entry,
            index + 1 == entries.len(),
            font_size,
            opacity
        ));
    }

    column.into()
}

/// Renders the modules that can still be added, each landing in `target`.
fn available_row<'a>(
    config: &Config,
    target: Section,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    let custom = config
        .custom_modules
        .iter()
        .map(|module| module.name.clone())
        .collect::<Vec<_>>();

    let mut row = Row::new()
        .spacing(ROW_GAP_EM * font_size * 0.5)
        .width(Length::Fill);

    for module in available(&config.modules, &custom) {
        row = row.push(choice_button(
            module.as_str().to_owned(),
            Message::EditLayout(LayoutEdit::Add {
                section: target,
                module:  module.clone()
            }),
            false,
            font_size * 0.9,
            opacity
        ));
    }

    Column::new()
        .push(text("Add to the left section").size(font_size))
        .push(row)
        .spacing(SECTION_GAP_EM * font_size)
        .width(Length::Fill)
        .into()
}

/// Renders the module editor against the running `config`.
pub(super) fn view(config: &Config, opacity: f32, font_size: f32) -> Element<'_, Message> {
    let mut page = Column::new()
        .spacing(PAGE_GAP_EM * font_size)
        .width(Length::Fill);

    for section in Section::ALL {
        let entries = match section {
            Section::Left => &config.modules.left,
            Section::Center => &config.modules.center,
            Section::Right => &config.modules.right
        };

        page = page.push(section_column(section, entries, font_size, opacity));
    }

    page.push(available_row(config, Section::Left, font_size, opacity))
        .push(
            text("↑ ↓ reorder · → move to the next section · join and split make islands")
                .size(font_size * 0.85)
        )
        .into()
}
