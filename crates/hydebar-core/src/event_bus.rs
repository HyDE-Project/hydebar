use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{Arc, Mutex}
};

use tokio::sync::Notify;

use crate::modules;

/// Time the receiver keeps collecting after the first wakeup.
///
/// Bursty producers such as the network backend publish several events in a
/// row; batching them into a single UI update keeps the redraw rate below the
/// refresh rate instead of repainting once per event.
const COALESCE_WINDOW: std::time::Duration = std::time::Duration::from_millis(8);

#[derive(Debug, Clone)]
#[non_exhaustive]
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

    /// Key identifying a whole-state message an older queued one yields to.
    ///
    /// The chattiest producers publish snapshots, not increments: a fresh
    /// workspace list, a fresh title, a fresh metrics sample each carry the
    /// whole truth, so a queue holding two of the same kind delivers stale
    /// state first and wastes a repaint on it. Incremental messages — the
    /// tray above all — have no key and are never replaced.
    const fn snapshot_key(&self) -> Option<u8> {
        match self {
            Self::Module(ModuleEvent::Workspaces(
                modules::workspaces::Message::WorkspacesChanged(_)
            )) => Some(0),
            Self::Module(ModuleEvent::WindowTitle(
                modules::window_title::Message::TitleChanged(_)
            )) => Some(1),
            Self::Module(ModuleEvent::SystemInfo(modules::system_info::Message::Sampled(_))) => {
                Some(2)
            }
            _ => None
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ModuleEvent {
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::TrayMessage),
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
struct EventBusInner {
    queue:     Mutex<VecDeque<BusEvent>>,
    capacity:  usize,
    /// Wakes the receiver as soon as a producer enqueues an event, so the UI
    /// never has to poll the queue on a timer.
    delivered: Notify
}

impl EventBusInner {
    fn new(capacity: NonZeroUsize) -> Self {
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
    fn queue(&self) -> std::sync::MutexGuard<'_, VecDeque<BusEvent>> {
        self.queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Appends `event`, replacing its stale twin or the oldest entry if full.
    ///
    /// Publishing never fails: a snapshot overwrites the stale snapshot of
    /// its kind in place, an overflowing queue first evicts the oldest
    /// replaceable snapshot — dropping it is lossless, a fresher one exists —
    /// and only then the oldest event of all. The bar keeps drawing the
    /// latest truth instead of punishing producers for a stalled consumer.
    fn enqueue(&self, event: BusEvent) {
        let mut queue = self.queue();

        if let Some(key) = event.snapshot_key()
            && let Some(stale) = queue
                .iter_mut()
                .find(|queued| queued.snapshot_key() == Some(key))
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

        if queue.len() >= self.capacity {
            let evicted = queue
                .iter()
                .position(|queued| queued.snapshot_key().is_some())
                .unwrap_or(0);

            if let Some(dropped) = queue.remove(evicted) {
                log::warn!(
                    "event bus at capacity {}: evicted {}",
                    self.capacity,
                    match dropped.snapshot_key() {
                        Some(_) => "a stale snapshot",
                        None => "the oldest event"
                    }
                );
            }
        }

        queue.push_back(event);
        drop(queue);

        self.delivered.notify_one();
    }

    /// Removes and returns every queued event.
    fn drain(&self) -> Vec<BusEvent> {
        self.queue().drain(..).collect()
    }
}

#[derive(Debug, Clone)]
pub struct EventBus {
    inner: Arc<EventBusInner>
}

impl EventBus {
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            inner: Arc::new(EventBusInner::new(capacity))
        }
    }

    #[must_use]
    pub fn sender(&self) -> EventSender {
        EventSender {
            inner: Arc::clone(&self.inner)
        }
    }

    #[must_use]
    pub fn receiver(&self) -> EventReceiver {
        EventReceiver {
            inner: Arc::clone(&self.inner)
        }
    }

    /// Appends `event` unless it coalesces with the queue tail.
    pub fn publish(&self, event: BusEvent) {
        self.inner.enqueue(event);
    }

    /// Removes and returns every queued event.
    #[must_use]
    pub fn drain(&self) -> Vec<BusEvent> {
        self.inner.drain()
    }
}

#[derive(Debug, Clone)]
pub struct EventSender {
    inner: Arc<EventBusInner>
}

impl EventSender {
    /// Appends `event` unless it coalesces with the queue tail.
    pub fn send(&self, event: BusEvent) {
        self.inner.enqueue(event);
    }
}

#[derive(Debug, Clone)]
pub struct EventReceiver {
    inner: Arc<EventBusInner>
}

impl EventReceiver {
    /// Removes and returns the front event, when one is queued.
    pub fn try_recv(&mut self) -> Option<BusEvent> {
        self.inner.queue().pop_front()
    }

    /// Waits until at least one event is queued and returns the whole batch.
    ///
    /// The future stays parked while the bus is empty, so an idle shell issues
    /// no wakeups at all instead of draining the queue on a timer.
    pub async fn recv(&mut self) -> Vec<BusEvent> {
        loop {
            let delivered = self.inner.delivered.notified();

            let batch = self.inner.drain();
            if !batch.is_empty() {
                return batch;
            }

            delivered.await;
            tokio::time::sleep(COALESCE_WINDOW).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroUsize, time::Duration};

    use super::*;

    fn bus() -> EventBus {
        EventBus::new(NonZeroUsize::new(16).expect("capacity is non zero"))
    }

    #[tokio::test]
    async fn a_fresh_snapshot_replaces_its_stale_twin_in_place() {
        let bus = bus();
        let mut receiver = bus.receiver();

        let snapshot = |active| {
            BusEvent::Module(ModuleEvent::Workspaces(
                crate::modules::workspaces::Message::WorkspacesChanged(
                    hydebar_proto::ports::hyprland::HyprlandWorkspaceSnapshot {
                        monitors:            Vec::new(),
                        workspaces:          Vec::new(),
                        active_workspace_id: Some(active)
                    }
                )
            ))
        };

        bus.publish(snapshot(1));
        bus.publish(BusEvent::PopupToggle);
        bus.publish(snapshot(2));

        let batch = receiver.recv().await;

        assert_eq!(batch.len(), 2, "the stale snapshot must not survive");
        assert!(matches!(
            &batch[0],
            BusEvent::Module(ModuleEvent::Workspaces(
                crate::modules::workspaces::Message::WorkspacesChanged(snapshot)
            )) if snapshot.active_workspace_id == Some(2)
        ));
    }

    #[tokio::test]
    async fn recv_returns_pending_events_without_waiting() {
        let bus = bus();
        let mut receiver = bus.receiver();
        bus.publish(BusEvent::Redraw);

        let batch = receiver.recv().await;

        assert_eq!(batch.len(), 1);
    }

    #[tokio::test]
    async fn recv_parks_until_a_producer_publishes() {
        let bus = bus();
        let mut receiver = bus.receiver();
        let sender = bus.sender();

        let waiting = tokio::spawn(async move { receiver.recv().await });

        assert!(!waiting.is_finished());

        sender.send(BusEvent::PopupToggle);

        let batch = tokio::time::timeout(Duration::from_secs(1), waiting)
            .await
            .expect("receiver woke up")
            .expect("task did not panic");

        assert!(matches!(batch.as_slice(), [BusEvent::PopupToggle]));
    }

    #[tokio::test]
    async fn recv_drains_the_whole_batch() {
        let bus = bus();
        let mut receiver = bus.receiver();
        bus.publish(BusEvent::Redraw);
        bus.publish(BusEvent::PopupToggle);

        let batch = receiver.recv().await;

        assert_eq!(batch.len(), 2);
        assert!(bus.drain().is_empty());
    }

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
