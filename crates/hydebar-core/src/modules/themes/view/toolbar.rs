//! The row of controls above the installed grid: the one fetch that updates
//! every theme, and the layout flip.

use iced::Element;

use crate::modules::themes::Message;

/// The row offering the one fetch that updates every installed theme.
pub(super) fn update_all_row<'a>(
    busy: bool,
    list_layout: bool,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    use iced::widget::{Row, button};

    use crate::{
        components::{
            icons::{Icons, icon_raw},
            page::style,
            scale
        },
        style::ghost_button_style
    };

    let control = style::control_size(font_size);
    let mut update = button(
        Row::new()
            .push(icon_raw(Icons::Refresh.default_glyph().to_owned()))
            .push(crate::components::text::text("Update all").size(control))
            .spacing(scale::icon_gap())
            .align_y(iced::Alignment::Center)
    )
    .padding(control * 0.25)
    .style(ghost_button_style(opacity));

    if !busy {
        update = update.on_press(Message::Update(None));
    }

    let layout_glyph = if list_layout {
        Icons::ViewGrid.default_glyph()
    } else {
        Icons::ViewList.default_glyph()
    };
    let layout = button(icon_raw(layout_glyph.to_owned()))
        .padding(control * 0.25)
        .style(ghost_button_style(opacity))
        .on_press(Message::ToggleLayout);

    Row::new()
        .push(update)
        .push(iced::widget::Space::new().width(iced::Length::Fill))
        .push(layout)
        .align_y(iced::Alignment::Center)
        .width(iced::Length::Fill)
        .into()
}
