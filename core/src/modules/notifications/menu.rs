//! Drawing of the notification center menu.

use iced::{
    Alignment, Element,
    widget::{Column, Row, button, container, scrollable}
};

use super::{Notifications, NotificationsMessage};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    services::notifications::Notification
};

impl Notifications {
    /// Render notification center menu popup.
    ///
    /// The header carries the DND toggle, the notification list scrolls
    /// under it.
    #[must_use]
    pub fn menu_view(
        &self,
        _opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, NotificationsMessage> {
        if self.service.is_none() {
            return text("Loading notifications...").into();
        }

        let notifications = &self.list;
        let is_dnd = self.dnd;

        let mut content = Column::new()
            .spacing(scale::scaled(8.0))
            .padding(scale::scaled(12.0));

        let header = Row::new()
            .push(text("Notifications").size(scale::scaled(16.0)))
            .push(
                button(text(if is_dnd { "DND: ON" } else { "DND: OFF" }))
                    .on_press(NotificationsMessage::ToggleDND)
            )
            .push(button(text("Clear All")).on_press(NotificationsMessage::ClearAll))
            .spacing(scale::scaled(8.0))
            .align_y(Alignment::Center);

        content = content.push(header);

        if notifications.is_empty() {
            content = content.push(text("No notifications").size(scale::scaled(14.0)));
        } else {
            let mut list = Column::new().spacing(scale::scaled(4.0));

            for notification in notifications {
                list = list.push(notification_item(notification, icons));
            }

            content = content.push(scrollable(list).height(scale::scaled(300.0)));
        }

        container(content)
            .style(move |theme| container::Style {
                background: Some(theme.palette().background.into()),
                border: iced::Border {
                    color:  theme.palette().primary,
                    width:  1.0,
                    radius: 8.0.into()
                },
                text_color: Some(theme.palette().text),
                ..Default::default()
            })
            .into()
    }
}

/// Render a single notification item.
fn notification_item<M>(notification: &Notification, icons: &IconTheme) -> Element<'static, M>
where
    M: 'static + Clone + From<NotificationsMessage>
{
    let summary = text(notification.summary.clone()).size(scale::scaled(14.0));
    let body = text(notification.body.clone()).size(scale::scaled(12.0));

    let content = Column::new()
        .push(
            Row::new()
                .push(summary)
                .push(
                    button(icon(icons, Icons::Close))
                        .on_press(NotificationsMessage::Dismiss(notification.id).into())
                )
                .spacing(scale::scaled(8.0))
                .align_y(Alignment::Center)
        )
        .push(body)
        .spacing(scale::scaled(4.0));

    container(content)
        .padding(scale::scaled(8.0))
        .style(|theme| container::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
