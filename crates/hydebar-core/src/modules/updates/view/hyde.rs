//! The `HyDE` block of the menu: version, missing commits and the
//! update that takes them.

use iced::{
    Element, Length, Padding,
    widget::{Column, button, column, container, row, scrollable}
};

use super::{
    super::state::{HydeSnapshot, Message, Updates},
    widgets::{action_button, log_block, truncated}
};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon as icon_component},
        scale,
        text::text
    },
    style::ghost_button_style
};

/// What the bar knows about the `HyDE` installation itself.
///
/// Reads like the package block above it: a current clone is one quiet
/// line naming the branch it follows, a stale one unfolds into the
/// upstream commits it is missing and offers to take them. A running
/// update narrates right here, as the tail of what the installer prints,
/// instead of opening a terminal.
pub(super) fn hyde_section<'a>(
    snapshot: &'a HydeSnapshot,
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
                    Column::with_children(snapshot.commits.iter().map(|subject| {
                        text(truncated(subject, 48).into_owned())
                            .size(scale::scaled(10.0))
                            .width(Length::Fill)
                            .into()
                    }))
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
