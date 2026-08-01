//! Small widgets the menu blocks share: buttons, log tails and the
//! truncation they all spell the same way.

use std::borrow::Cow;

use iced::{
    Element, Length,
    widget::{Column, button, container, row}
};

use super::super::state::{CheckState, Message, Updates};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon as icon_component},
        scale,
        text::text
    },
    style::ghost_button_style
};

/// The tail of what a running update printed, as quiet small lines.
pub(super) fn log_block(lines: &[String]) -> Element<'_, Message> {
    container(
        Column::with_children(lines.iter().map(|line| {
            text(truncated(line, 60).into_owned())
                .size(scale::scaled(10.0))
                .width(Length::Fill)
                .into()
        }))
        .spacing(scale::scaled(2.0))
    )
    .padding([scale::scaled(4.0), scale::scaled(8.0)])
    .into()
}

/// A full-width menu action; `message: None` draws it disabled, which is
/// how one update keeps the other from starting beside it.
pub(super) fn action_button(
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

pub(super) fn check_now_button<'a>(
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

pub(super) fn truncated(value: &str, max: usize) -> Cow<'_, str> {
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
