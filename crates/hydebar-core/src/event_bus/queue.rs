//! The events on the bus and the queue that tames their bursts.
//!
//! The chattiest producers publish snapshots, not increments, so the queue
//! replaces a stale snapshot with its fresh twin in place, folds a duplicate
//! redraw into the tail, and when full lets go of its oldest entry. The bar
//! keeps drawing the latest truth instead of punishing producers for a
//! stalled consumer.

use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{Arc, Mutex}
};

use tokio::sync::Notify;

use crate::modules;

#[derive(Debug, Clone)]
pub enum BusEvent {
    Redraw,
    PopupToggle,
    Module(ModuleEvent)
}

impl BusEvent {
    const fn is_coalescable_with(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Redraw, Self::Redraw) | (Self::PopupToggle, Self::PopupToggle)
        )
    }

    /// The snapshot family this message belongs to, when it is one.
    ///
    /// The chattiest producers publish snapshots, not increments: a fresh
    /// workspace list, a fresh title, a fresh metrics sample each carry the
    /// whole truth, so a queue holding two of the same kind delivers stale
    /// state first and wastes a repaint on it. Incremental messages — the
    /// tray above all — have no family and are never replaced.
    const fn snapshot_kind(&self) -> Option<SnapshotKind> {
        match self {
            Self::Module(ModuleEvent::Workspaces(
                modules::workspaces::Message::WorkspacesChanged(_)
            )) => Some(SnapshotKind::Workspaces),
            Self::Module(ModuleEvent::WindowTitle(
                modules::window_title::Message::TitleChanged(_)
            )) => Some(SnapshotKind::WindowTitle),
            Self::Module(ModuleEvent::SystemInfo(modules::system_info::Message::Sampled(_))) => {
                Some(SnapshotKind::SystemInfo)
            }
            Self::Module(ModuleEvent::Taskbar(modules::taskbar::Message::ClientsChanged(_))) => {
                Some(SnapshotKind::Taskbar)
            }
            _ => None
        }
    }
}

/// The whole-state message families a fresh publication replaces in place.
///
/// One named variant per snapshot producer keeps [`BusEvent::snapshot_kind`]
/// the single spot naming them: a new chatty producer adds a variant here and
/// one match arm there, instead of inventing the next free number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapshotKind {
    Workspaces,
    WindowTitle,
    SystemInfo,
    Taskbar
}

#[derive(Debug, Clone)]
pub enum ModuleEvent {
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::TrayMessage),
    Taskbar(modules::taskbar::Message),
    Clock(modules::clock::Message),
    Battery(modules::battery::Message),
    Privacy(modules::privacy::PrivacyMessage),
    ControlCenter(modules::control_center::Message),
    MediaPlayer(modules::media_player::Message),
    Notifications(modules::notifications::NotificationsMessage),
    Weather(modules::weather::Message),
    Custom {
        name:    Arc<str>,
        message: modules::custom_module::Message
    }
}

#[derive(Debug)]
pub(super) struct EventBusInner {
    queue:                Mutex<VecDeque<BusEvent>>,
    capacity:             usize,
    /// Wakes the receiver as soon as a producer enqueues an event, so the UI
    /// never has to poll the queue on a timer.
    pub(super) delivered: Notify
}

impl EventBusInner {
    pub(super) fn new(capacity: NonZeroUsize) -> Self {
        Self {
            queue:     Mutex::new(VecDeque::with_capacity(capacity.get())),
            capacity:  capacity.get(),
            delivered: Notify::new()
        }
    }

    /// Takes the queue lock, recovering it from a producer's panic.
    ///
    /// The queue holds no invariant spanning two operations — every push,
    /// replace and drain leaves it whole — so the state behind a poisoned
    /// lock is still consistent and abandoning the whole bus over one
    /// producer's panic would trade a recoverable hiccup for a dead UI.
    pub(super) fn queue(&self) -> std::sync::MutexGuard<'_, VecDeque<BusEvent>> {
        self.queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Appends `event`, replacing its stale twin or the oldest entry if full.
    ///
    /// Publishing never fails: a snapshot overwrites the stale snapshot of
    /// its kind in place, and an overflowing queue lets go of its oldest
    /// entry — any same-kind twin was already replaced above, so whatever
    /// stands in the queue is unique and age is the only honest tiebreak.
    /// The bar keeps drawing the latest truth instead of punishing
    /// producers for a stalled consumer.
    pub(super) fn enqueue(&self, event: BusEvent) {
        let mut queue = self.queue();

        if let Some(kind) = event.snapshot_kind()
            && let Some(stale) = queue
                .iter_mut()
                .find(|queued| queued.snapshot_kind() == Some(kind))
        {
            *stale = event;
            drop(queue);
            self.delivered.notify_one();

            return;
        }

        if let Some(last) = queue.back()
            && event.is_coalescable_with(last)
        {
            return;
        }

        if queue.len() >= self.capacity && queue.pop_front().is_some() {
            log::warn!(
                "event bus at capacity {}: the oldest event was evicted",
                self.capacity
            );
        }

        queue.push_back(event);
        drop(queue);

        self.delivered.notify_one();
    }

    /// Removes and returns every queued event.
    pub(super) fn drain(&self) -> Vec<BusEvent> {
        self.queue().drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::event_bus::{EventBus, ModuleEvent};

    fn workspace_snapshot(active: i32) -> BusEvent {
        BusEvent::Module(ModuleEvent::Workspaces(
            crate::modules::workspaces::Message::WorkspacesChanged(
                hydebar_proto::ports::hyprland::HyprlandWorkspaceSnapshot {
                    monitors:            Vec::new(),
                    workspaces:          Vec::new(),
                    active_workspace_id: Some(active)
                }
            )
        ))
    }

    #[test]
    fn a_full_queue_evicts_the_oldest_event_and_accepts_the_newest() {
        let bus = EventBus::new(NonZeroUsize::new(1).expect("capacity is non zero"));
        bus.publish(BusEvent::Redraw);

        bus.publish(BusEvent::PopupToggle);

        let queued = bus.drain();
        assert!(
            matches!(queued.as_slice(), [BusEvent::PopupToggle]),
            "the newest event must survive an overflow"
        );
    }

    #[test]
    fn a_full_queue_prefers_evicting_a_stale_snapshot_over_a_one_off_event() {
        let bus = EventBus::new(NonZeroUsize::new(2).expect("capacity is non zero"));

        bus.publish(workspace_snapshot(1));
        bus.publish(BusEvent::PopupToggle);

        bus.publish(BusEvent::Module(ModuleEvent::Clock(
            crate::modules::clock::Message::Update
        )));

        let queued = bus.drain();
        assert_eq!(queued.len(), 2);
        assert!(
            matches!(
                queued.as_slice(),
                [
                    BusEvent::PopupToggle,
                    BusEvent::Module(ModuleEvent::Clock(crate::modules::clock::Message::Update))
                ]
            ),
            "the one-off toggle must outlive the replaceable snapshot"
        );
    }
}
