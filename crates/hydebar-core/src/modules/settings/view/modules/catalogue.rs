//! Catalogue of the modules that are not on the bar yet.

use hydebar_proto::config::Config;
use iced::Element;

use crate::{
    components::page::{
        metrics::{chip_cell_width, wrap_chips_into_rows},
        style,
        widgets::{chip, grid, group, note}
    },
    modules::settings::{
        Message,
        layout::{LayoutEdit, Section, available}
    }
};

/// Renders the modules that can still be added.
///
/// The chips wrap onto as many rows as the width allows, so a long
/// catalogue never runs past the edge of the window.
pub(super) fn catalogue<'a>(
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

    let cell = chip_cell_width(&labels, font_size);
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
                opacity,
                Some(cell)
            ));
        }

        block = block.push(row);
    }

    block.into()
}
