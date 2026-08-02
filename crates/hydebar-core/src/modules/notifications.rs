//! The notification center: a bell in the bar, the served store behind it.
//!
//! One folder, three rooms: [`state`] folds service events into a rendered
//! snapshot, [`menu`] draws the notification center popup and [`module`]
//! wires the module to the bar. The root holds the state the rooms share.

use crate::{
    ModuleEventSender,
    services::{
        ServiceEvent,
        notifications::{Notification, NotificationsService}
    }
};

mod menu;
mod module;
mod state;

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
