//! What the updates module draws: the bar entry and the menu around it.

use iced::{
    Alignment, Element, Length, SurfaceId as Id,
    widget::{column, container, row, rule}
};

use super::state::{CheckState, Message, Updates};
use crate::components::{
    icons::{IconTheme, Icons, icon as icon_component},
    scale,
    text::text
};

mod hyde;
mod list;
mod widgets;

use widgets::{action_button, check_now_button, log_block};

pub(super) fn menu_view<'a>(
    updates: &'a Updates,
    _id: Id,
    opacity: f32,
    icons: &IconTheme
) -> Element<'a, Message> {
    let packages: Element<'a, Message> = if updates.updates().is_empty() {
        container(text("Up to date ;)"))
            .padding([scale::scaled(8.0), scale::scaled(8.0)])
            .into()
    } else {
        list::build_updates_list(updates, opacity, icons)
    };

    let mut menu = column!(packages, rule::horizontal(1)).spacing(scale::scaled(4.0));

    if let Some(snapshot) = updates.hyde() {
        menu = menu
            .push(hyde::hyde_section(snapshot, updates, opacity, icons))
            .push(rule::horizontal(1));
    }

    if updates.is_applying() {
        menu = menu.push(
            container(row!(
                text("Updating").width(Length::Fill),
                icon_component(icons, Icons::Refresh)
            ))
            .padding([scale::scaled(8.0), scale::scaled(8.0)])
        );
    } else {
        menu = menu.push(action_button(
            "Update",
            (!updates.is_hyde_updating()).then_some(Message::Update),
            opacity
        ));
    }

    if !updates.apply_log().is_empty() {
        menu = menu.push(log_block(updates.apply_log()));
    }

    menu.push(check_now_button(updates, opacity, icons)).into()
}

pub(super) fn icon(
    state: &CheckState,
    update_count: usize,
    hyde_pending: usize,
    count: Element<'static, Message>,
    icons: &IconTheme
) -> Element<'static, Message> {
    let icon = match state {
        CheckState::Checking => Icons::Refresh,
        CheckState::Unavailable => Icons::NoUpdatesAvailable,
        CheckState::Ready if update_count == 0 && hyde_pending == 0 => Icons::NoUpdatesAvailable,
        CheckState::Ready => Icons::UpdatesAvailable
    };

    let mut content = row!(container(icon_component(icons, icon)))
        .align_y(Alignment::Center)
        .spacing(scale::icon_gap());

    if update_count > 0 {
        content = content.push(count);
    }

    content.into()
}
