//! Notification popups: raising, expiry and the surface that holds them.

use hydebar_core::modules;
use iced::Task;

use super::super::super::state::{App, Message};

impl App {
    /// Shows a popup for a notification that has just arrived.
    ///
    /// A notification the user dismissed is taken off the screen at once, so
    /// clearing the centre also clears what is floating above it.
    fn raise_popup(&mut self, message: &modules::notifications::NotificationsMessage) {
        use hydebar_core::{
            notifications_popup::Popup,
            services::{ServiceEvent, notifications::NotificationEvent}
        };

        match message {
            modules::notifications::NotificationsMessage::Event(ServiceEvent::Update(
                NotificationEvent::Received(notification)
            )) => {
                if self.config.notifications.source.hands_to_compositor() {
                    self.hand_to_compositor(notification);
                    return;
                }

                if !self.config.notifications.source.draws_popups() {
                    return;
                }

                self.notification_popups
                    .push(Popup::new(notification, std::time::Instant::now()));
                hydebar_core::notifications_popup::prune(
                    &mut self.notification_popups,
                    std::time::Instant::now()
                );
            }
            modules::notifications::NotificationsMessage::Dismiss(id) => {
                self.notification_popups.retain(|popup| popup.id != *id);
            }
            modules::notifications::NotificationsMessage::ClearAll => {
                self.notification_popups.clear();
            }
            _ => {}
        }
    }

    /// Hands a notification to the compositor, painted with the theme.
    ///
    /// Nothing is drawn by the bar in this mode: the compositor shows its own
    /// notification, which is what a session that wants no extra pieces asked
    /// for.
    fn hand_to_compositor(
        &self,
        notification: &hydebar_core::services::notifications::Notification
    ) {
        use hydebar_core::services::{
            hyprland_notify::{Notice, compositor_color, notify},
            notifications::Urgency
        };

        let notice = match notification.urgency {
            Urgency::Low => Notice::Hint,
            Urgency::Normal => Notice::Info,
            Urgency::Critical => Notice::Error
        };

        let color = compositor_color(self.appearance().primary_color);
        let duration = hydebar_core::notifications_popup::lifetime_for(&notification.urgency);
        let message = if notification.body.is_empty() {
            notification.summary.clone()
        } else {
            format!("{}: {}", notification.summary, notification.body)
        };

        #[expect(
            clippy::cast_possible_truncation,
            reason = "popup lifetimes are far below u32 milliseconds"
        )]
        notify(
            notice,
            duration.as_millis() as u32,
            &color,
            self.appearance().font_size_px(),
            &message
        );
    }

    /// Grows or shrinks the notification surface to what the popups need.
    pub(crate) fn fit_notification_surface(&mut self) -> Task<Message> {
        let height = hydebar_core::notifications_popup::surface_height(
            &self.notification_popups,
            self.appearance()
        );

        self.outputs.resize_notifications(height)
    }

    /// Takes down the popups whose time is up.
    pub(super) fn expire_popups(&mut self) -> Task<Message> {
        let before = self.notification_popups.len();

        hydebar_core::notifications_popup::prune(
            &mut self.notification_popups,
            std::time::Instant::now()
        );

        if before == self.notification_popups.len() {
            Task::none()
        } else {
            self.fit_notification_surface()
        }
    }

    /// Forwards a notifications message and refits the popup surface.
    pub(super) fn update_notifications(
        &mut self,
        msg: modules::notifications::NotificationsMessage
    ) -> Task<Message> {
        let popups_before = self.notification_popups.len();

        self.raise_popup(&msg);
        self.notifications.update(msg);

        if popups_before == self.notification_popups.len() {
            Task::none()
        } else {
            self.fit_notification_surface()
        }
    }
}
