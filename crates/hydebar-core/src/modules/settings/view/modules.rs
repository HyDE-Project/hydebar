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

use super::{
    metrics::{button_row_width, button_width, text_width},
    widgets::{ROW_GAP_EM, choice_button}
};
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

/// Splits `labels` into rows that fit inside `available` pixels.
///
/// Widths are estimated rather than measured: the layout engine reports sizes
/// only after the fact, and a row that wraps one entry too early costs far less
/// than one that runs off the window.
fn wrap_into_rows(labels: &[String], available: f32, font_size: f32, gap: f32) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut used = 0.0;

    for (index, label) in labels.iter().enumerate() {
        let width = button_width(label, font_size);
        let fits = used + width <= available;

        match rows.last_mut() {
            Some(row) if fits => {
                row.push(index);
                used += width + gap;
            }
            _ => {
                rows.push(vec![index]);
                used = width + gap;
            }
        }
    }

    rows
}

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
///
/// The buttons wrap onto as many rows as `available` pixels of width call for,
/// so a long catalogue never runs past the edge of the window.
fn available_rows<'a>(
    config: &Config,
    target: Section,
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
    let labels = modules
        .iter()
        .map(|module| module.as_str().to_owned())
        .collect::<Vec<_>>();

    let button_font = font_size * 0.9;
    let gap = ROW_GAP_EM * font_size * 0.5;

    let mut column = Column::new()
        .push(text("Add to the left section").size(font_size))
        .spacing(SECTION_GAP_EM * font_size)
        .width(Length::Fill);

    for indices in wrap_into_rows(&labels, available_width, button_font, gap) {
        let mut row = Row::new().spacing(gap).width(Length::Fill);

        for index in indices {
            row = row.push(choice_button(
                labels[index].clone(),
                Message::EditLayout(LayoutEdit::Add {
                    section: target,
                    module:  modules[index].clone()
                }),
                false,
                button_font,
                opacity
            ));
        }

        column = column.push(row);
    }

    column.into()
}

/// Renders the module editor against the running `config`, given the width its
/// content may spend.
pub(super) fn view(
    config: &Config,
    opacity: f32,
    font_size: f32,
    available_width: f32
) -> Element<'_, Message> {
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

    page.push(available_rows(
        config,
        Section::Left,
        font_size,
        opacity,
        available_width
    ))
    .push(
        text("↑ ↓ reorder · → move to the next section · join and split make islands")
            .size(font_size * 0.85)
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_that_fits_stays_a_single_row() {
        let labels = vec!["Clock".to_owned(), "Tray".to_owned()];

        assert_eq!(wrap_into_rows(&labels, 1000.0, 10.0, 4.0), vec![vec![0, 1]]);
    }

    #[test]
    fn a_long_catalogue_wraps_onto_several_rows() {
        let labels = (0..8).map(|i| format!("Module{i}")).collect::<Vec<_>>();

        let rows = wrap_into_rows(&labels, 200.0, 10.0, 4.0);

        assert!(rows.len() > 1);
        assert_eq!(rows.iter().map(Vec::len).sum::<usize>(), labels.len());
    }

    #[test]
    fn an_entry_wider_than_the_row_still_gets_a_row() {
        let labels = vec!["AnExtremelyLongModuleName".to_owned()];

        assert_eq!(wrap_into_rows(&labels, 10.0, 10.0, 4.0), vec![vec![0]]);
    }

    #[test]
    fn an_empty_catalogue_yields_no_rows() {
        assert!(wrap_into_rows(&[], 500.0, 10.0, 4.0).is_empty());
    }

    #[test]
    fn a_wider_label_is_expected_to_take_more_room() {
        assert!(button_width("Clock", 10.0) < button_width("KeyboardLayout", 10.0));
    }
}

/// Buttons every entry row carries, at their widest.
const ENTRY_CONTROLS: [&str; 6] = [
    "\u{2191}", "\u{2193}", "\u{2192}", "split", "join", "\u{2715}"
];

/// Longest row of this page, which is how wide the window has to be.
///
/// The catalogue is left out on purpose: it wraps into whatever width the
/// entries settle on, so letting it vote would make the window as wide as the
/// list of every module the bar ships.
#[must_use]
pub(super) fn desired_width(config: &Config, font_size: f32) -> f32 {
    let gap = ROW_GAP_EM * font_size;
    let controls = button_row_width(ENTRY_CONTROLS.into_iter(), font_size, gap * 0.5);

    Section::ALL
        .into_iter()
        .flat_map(|section| match section {
            Section::Left => config.modules.left.iter(),
            Section::Center => config.modules.center.iter(),
            Section::Right => config.modules.right.iter()
        })
        .map(|entry| text_width(&entry_label(entry), font_size) + gap + controls)
        .fold(0.0_f32, f32::max)
}
