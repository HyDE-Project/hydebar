//! The package list of the menu, folded behind its header button.

use iced::{
    Element, Length, Padding,
    alignment::Horizontal,
    widget::{Column, button, column, container, row, scrollable}
};

use super::{
    super::state::{Message, Update, Updates},
    widgets::truncated
};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon as icon_component},
        scale,
        text::text
    },
    style::ghost_button_style
};

pub(super) fn build_updates_list<'a>(
    updates: &'a Updates,
    opacity: f32,
    icons: &IconTheme
) -> Element<'a, Message> {
    let mut elements = column!(
        button(row!(
            text(format!("{} Updates available", updates.updates().len())).width(Length::Fill),
            icon_component(
                icons,
                if updates.is_updates_list_open() {
                    Icons::MenuClosed
                } else {
                    Icons::MenuOpen
                }
            )
        ))
        .style(ghost_button_style(opacity))
        .padding([scale::scaled(8.0), scale::scaled(8.0)])
        .on_press(Message::ToggleUpdatesList)
        .width(Length::Fill),
    );

    if updates.is_updates_list_open() {
        elements = elements.push(
            container(scrollable(
                Column::with_children(
                    updates
                        .updates()
                        .iter()
                        .map(|update| build_update_entry(update))
                )
                .padding(Padding::ZERO.right(16))
                .spacing(scale::scaled(4.0))
            ))
            .padding([scale::scaled(8.0), scale::scaled(0.0)])
            .max_height(300)
        );
    }

    elements.into()
}

fn build_update_entry(update: &Update) -> Element<'_, Message> {
    column!(
        text(update.package.as_str())
            .size(scale::scaled(10.0))
            .width(Length::Fill),
        text(format!(
            "{} -> {}",
            truncated(&update.from, 18),
            truncated(&update.to, 18)
        ))
        .width(Length::Fill)
        .align_x(Horizontal::Right)
        .size(scale::scaled(10.0)),
    )
    .into()
}
