use std::borrow::Cow;

use iced::{
    Alignment, Element, Length, Padding, SurfaceId as Id,
    alignment::Horizontal,
    widget::{Column, button, column, container, row, rule, scrollable}
};

use super::state::{CheckState, Message, Updates};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon as icon_component},
        scale,
        text::text
    },
    style::ghost_button_style
};

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
        build_updates_list(updates, opacity, icons)
    };

    let mut menu = column!(packages, rule::horizontal(1)).spacing(scale::scaled(4.0));

    if let Some(snapshot) = updates.hyde() {
        menu = menu
            .push(hyde_section(snapshot, updates, opacity, icons))
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

/// The tail of what a running update printed, as quiet small lines.
fn log_block(lines: &[String]) -> Element<'_, Message> {
    container(
        Column::with_children(
            lines
                .iter()
                .map(|line| {
                    text(truncated(line, 60).into_owned())
                        .size(scale::scaled(10.0))
                        .width(Length::Fill)
                        .into()
                })
        )
        .spacing(scale::scaled(2.0))
    )
    .padding([scale::scaled(4.0), scale::scaled(8.0)])
    .into()
}

/// What the bar knows about the `HyDE` installation itself.
///
/// Reads like the package block above it: a current clone is one quiet
/// line naming the branch it follows, a stale one unfolds into the
/// upstream commits it is missing and offers to take them. A running
/// update narrates right here, as the tail of what the installer prints,
/// instead of opening a terminal.
fn hyde_section<'a>(
    snapshot: &'a super::state::HydeSnapshot,
    updates: &'a Updates,
    opacity: f32,
    icons: &IconTheme
) -> Element<'a, Message> {
    let branch = updates.hyde_branch_name();
    let mut section = column!().spacing(scale::scaled(4.0));

    if updates.is_hyde_updating() {
        section = section.push(
            container(row!(
                text(format!("HyDE · {branch} · updating")).width(Length::Fill),
                icon_component(icons, Icons::Refresh)
            ))
            .padding([scale::scaled(8.0), scale::scaled(8.0)])
        );
    } else if snapshot.commits.is_empty() {
        section = section.push(
            container(text(format!(
                "HyDE {} · {branch} · up to date",
                snapshot.version
            )))
            .padding([scale::scaled(8.0), scale::scaled(8.0)])
        );
    } else {
        section = section.push(
            button(row!(
                text(format!(
                    "HyDE {} · {branch} · {} commits behind",
                    snapshot.version,
                    snapshot.commits.len()
                ))
                .width(Length::Fill),
                icon_component(
                    icons,
                    if updates.is_hyde_list_open() {
                        Icons::MenuClosed
                    } else {
                        Icons::MenuOpen
                    }
                )
            ))
            .style(ghost_button_style(opacity))
            .padding([scale::scaled(8.0), scale::scaled(8.0)])
            .on_press(Message::ToggleHydeList)
            .width(Length::Fill)
        );

        if updates.is_hyde_list_open() {
            section = section.push(
                container(scrollable(
                    Column::with_children(
                        snapshot
                            .commits
                            .iter()
                            .map(|subject| {
                                text(truncated(subject, 48).into_owned())
                                    .size(scale::scaled(10.0))
                                    .width(Length::Fill)
                                    .into()
                            })
                    )
                    .padding(Padding::ZERO.right(16))
                    .spacing(scale::scaled(4.0))
                ))
                .padding([scale::scaled(8.0), scale::scaled(0.0)])
                .max_height(300)
            );
        }

        section = section.push(action_button(
            "Update HyDE",
            (!updates.is_applying()).then_some(Message::UpdateHyde),
            opacity
        ));
    }

    if !updates.hyde_log().is_empty() {
        section = section.push(log_block(updates.hyde_log()));
    }

    section.into()
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
        CheckState::Ready if update_count == 0 && hyde_pending == 0 => {
            Icons::NoUpdatesAvailable
        }
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

fn build_updates_list<'a>(
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

fn build_update_entry(update: &super::state::Update) -> Element<'_, Message> {
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

/// A full-width menu action; `message: None` draws it disabled, which is
/// how one update keeps the other from starting beside it.
fn action_button(
    label: &str,
    message: Option<Message>,
    opacity: f32
) -> iced::widget::Button<'_, Message> {
    button(label)
        .style(ghost_button_style(opacity))
        .padding([scale::scaled(8.0), scale::scaled(8.0)])
        .on_press_maybe(message)
        .width(Length::Fill)
}

fn check_now_button<'a>(
    updates: &'a Updates,
    opacity: f32,
    icons: &IconTheme
) -> iced::widget::Button<'a, Message> {
    let mut content = row!(text("Check now").width(Length::Fill));

    if matches!(updates.state(), CheckState::Checking) {
        content = content.push(icon_component(icons, Icons::Refresh));
    }

    button(content)
        .style(ghost_button_style(opacity))
        .padding([scale::scaled(8.0), scale::scaled(8.0)])
        .on_press(Message::CheckNow)
        .width(Length::Fill)
}

fn truncated(value: &str, max: usize) -> Cow<'_, str> {
    if value.chars().count() <= max {
        Cow::Borrowed(value)
    } else {
        Cow::Owned(value.chars().take(max).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_returns_borrowed_when_short_enough() {
        let value = "short";

        assert!(matches!(truncated(value, 10), Cow::Borrowed("short")));
    }

    #[test]
    fn truncated_returns_owned_when_too_long() {
        let value = "averylongstring";

        let truncated = truncated(value, 5);

        assert!(matches!(truncated, Cow::Owned(ref owned) if owned == "avery"));
    }
}
