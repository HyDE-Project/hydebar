//! What the bar takes from the desktop around it.

use iced::Element;

use super::super::{HYDE_BRANCH, NOTIFICATIONS};
use crate::{
    components::{
        page::widgets::{choice_row, rows as row_stack},
        push_maybe::PushMaybe
    },
    config::{Config, HydeBranch, NotificationSource},
    modules::settings::Message
};

/// Rows of the desktop section, against the running `config`.
pub fn desktop_rows(config: &Config, font_size: f32, opacity: f32) -> Element<'_, Message> {
    row_stack(font_size)
        .push(choice_row(
            NOTIFICATIONS,
            NotificationSource::ALL
                .into_iter()
                .map(|source| {
                    (
                        source.label(),
                        source,
                        config.notifications.source == source
                    )
                })
                .collect(),
            Message::SetNotificationSource,
            font_size,
            opacity
        ))
        .push_maybe(config.updates.as_ref().map(|updates| {
            choice_row(
                HYDE_BRANCH,
                HydeBranch::ALL
                    .into_iter()
                    .map(|branch| (branch.label(), branch, updates.hyde_branch == branch))
                    .collect(),
                Message::SetHydeBranch,
                font_size,
                opacity
            )
        }))
        .into()
}
