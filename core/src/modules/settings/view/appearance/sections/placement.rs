//! Where the bar stands: its edge of the screen and its layer.

use iced::Element;

use crate::{
    components::page::widgets::{choice_row, rows as row_stack},
    config::{BarLayer, Config, Position},
    modules::settings::Message
};

/// Rows of the placement section, against the running `config`.
pub fn placement_rows(config: &Config, font_size: f32, opacity: f32) -> Element<'_, Message> {
    row_stack(font_size)
        .push(choice_row(
            "Position",
            vec![
                ("Top", Position::Top, config.position == Position::Top),
                (
                    "Bottom",
                    Position::Bottom,
                    config.position == Position::Bottom
                ),
            ],
            Message::SetPosition,
            font_size,
            opacity
        ))
        .push(choice_row(
            "Layer",
            vec![
                ("Bottom", BarLayer::Bottom, config.layer == BarLayer::Bottom),
                ("Top", BarLayer::Top, config.layer == BarLayer::Top),
                (
                    "Overlay",
                    BarLayer::Overlay,
                    config.layer == BarLayer::Overlay
                ),
            ],
            Message::SetLayer,
            font_size,
            opacity
        ))
        .into()
}
