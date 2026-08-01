use iced::{
    Alignment, Element,
    widget::{Column, Row, button, container, scrollable}
};
use log::error;

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, ModuleEventSender,
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    event_bus::ModuleEvent,
    menu::MenuType,
    services::{
        ReadOnlyService, ServiceEvent,
        notifications::{Notification, NotificationsService}
    }
};

/// Message emitted by the notifications module.
#[derive(Debug, Clone)]
pub enum NotificationsMessage {
    Event(ServiceEvent<NotificationsService>),
    Dismiss(u32),
    ClearAll,
    ToggleDND
}

/// UI module displaying notification center with bell icon.
#[derive(Debug, Default)]
pub struct Notifications {
    pub service: Option<NotificationsService>,
    sender:      Option<ModuleEventSender<NotificationsMessage>>,
    /// Notifications as of the last event, rendered without touching the
    /// store.
    ///
    /// The store sits behind a lock the notification server writes from; a
    /// view that read it directly would take that lock and deep-copy the
    /// list on every frame of the menu animation. The copy is made once per
    /// event instead, which is the only time it can change.
    list:        Vec<Notification>,
    /// Unread notifications as of the last event.
    unread:      usize,
    /// Whether do-not-disturb was on as of the last event.
    dnd:         bool
}

impl Notifications {
    /// Update the module state based on notification events.
    pub fn update(&mut self, message: NotificationsMessage) {
        match message {
            NotificationsMessage::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                }
                ServiceEvent::Update(data) => {
                    if let Some(notifications) = self.service.as_mut() {
                        notifications.update(data);
                    }
                }
                ServiceEvent::Error(error) => {
                    error!("Notifications service error: {error}");
                }
            },
            NotificationsMessage::Dismiss(id) => {
                if let Some(service) = self.service.as_mut() {
                    service.dismiss(id);
                }
            }
            NotificationsMessage::ClearAll => {
                if let Some(service) = self.service.as_mut() {
                    service.clear_all();
                }
            }
            NotificationsMessage::ToggleDND => {
                if let Some(service) = self.service.as_mut() {
                    service.toggle_dnd();
                }
            }
        }

        self.refresh_snapshot();
    }

    /// Re-reads the store once, after something actually changed.
    fn refresh_snapshot(&mut self) {
        let Some(service) = self.service.as_ref() else {
            return;
        };

        self.list = service.get_notifications();
        self.unread = service.unread_count();
        self.dnd = service.is_dnd();
    }

    /// Render notification center menu popup.
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

        // Header with DND toggle
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

        // Notification list
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

impl<M> Module<M> for Notifications
where
    M: 'static + Clone + From<NotificationsMessage>
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        let sender = ctx.module_sender(ModuleEvent::Notifications);
        self.sender = Some(sender);

        Ok(())
    }

    /// Forgets the served store once the bus moves to a separate daemon.
    ///
    /// The D-Bus server itself lives inside the [`subscription`] stream and
    /// dies with it; what survives here is a handle to the storage that
    /// server wrote and the snapshot rendered from it. Kept, they would show
    /// the last notifications of the previous source as if they were live.
    ///
    /// [`subscription`]: Module::subscription
    fn deregister(&mut self) {
        self.sender = None;
        self.service = None;
        self.list.clear();
        self.unread = 0;
        self.dnd = false;
    }

    fn subscription(&self) -> Option<iced::Subscription<M>> {
        use crate::services::ReadOnlyService;

        Some(
            crate::services::notifications::NotificationsService::subscribe()
                .map(NotificationsMessage::Event)
                .map(M::from)
        )
    }

    /// Render notification icon with unread count.
    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        let content = if self.unread > 0 {
            Row::new()
                .push(icon(icons, Icons::Bell))
                .push(text(self.unread))
                .spacing(scale::icon_gap())
                .align_y(Alignment::Center)
        } else {
            Row::new().push(icon(icons, Icons::Bell))
        };

        Some((
            container(content).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Notifications))
        ))
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
