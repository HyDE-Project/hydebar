use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{Arc, Mutex}
};

use masterror::AppError;
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
            Self::Module(ModuleEvent::SystemInfo(
                modules::system_info::Message::Sampled(_)
            )) => Some(2),
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

    /// Appends `event` unless it coalesces with the queue tail.
    ///
    /// # Errors
    ///
    /// Returns [`EventBusError::QueueFull`] when the queue reached its capacity
    /// and [`EventBusError::Poisoned`] when a producer panicked while holding
    /// the queue lock.
    fn enqueue(&self, event: BusEvent) -> Result<(), EventBusError> {
        let mut queue = self.queue.lock().map_err(|_| EventBusError::Poisoned)?;

        if let Some(key) = event.snapshot_key()
            && let Some(stale) = queue
                .iter_mut()
                .find(|queued| queued.snapshot_key() == Some(key))
        {
            *stale = event;
            drop(queue);
            self.delivered.notify_one();

            return Ok(());
        }

        if queue.len() >= self.capacity {
            return Err(EventBusError::QueueFull {
                capacity: self.capacity
            });
        }

        if let Some(last) = queue.back()
            && event.is_coalescable_with(last)
        {
            return Ok(());
        }

        queue.push_back(event);
        drop(queue);

        self.delivered.notify_one();

        Ok(())
    }

    /// Removes and returns every queued event.
    ///
    /// # Errors
    ///
    /// Returns [`EventBusError::Poisoned`] when a producer panicked while
    /// holding the queue lock.
    fn drain(&self) -> Result<Vec<BusEvent>, EventBusError> {
        let mut queue = self.queue.lock().map_err(|_| EventBusError::Poisoned)?;

        Ok(queue.drain(..).collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventBusError {
    QueueFull { capacity: usize },
    Poisoned
}

impl std::fmt::Display for EventBusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull {
                capacity
            } => {
                write!(f, "Event queue is full (capacity: {capacity})")
            }
            Self::Poisoned => write!(f, "Event queue state is poisoned")
        }
    }
}

impl std::error::Error for EventBusError {}

impl From<EventBusError> for AppError {
    fn from(err: EventBusError) -> Self {
        match err {
            EventBusError::QueueFull {
                ..
            } => Self::internal(err.to_string()),
            EventBusError::Poisoned => Self::internal(err.to_string())
        }
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

    pub fn publish(&self, event: BusEvent) -> Result<(), EventBusError> {
        self.inner.enqueue(event)
    }

    pub fn drain(&self) -> Result<Vec<BusEvent>, EventBusError> {
        self.inner.drain()
    }
}

#[derive(Debug, Clone)]
pub struct EventSender {
    inner: Arc<EventBusInner>
}

impl EventSender {
    pub fn try_send(&self, event: BusEvent) -> Result<(), EventBusError> {
        self.inner.enqueue(event)
    }
}

#[derive(Debug, Clone)]
pub struct EventReceiver {
    inner: Arc<EventBusInner>
}

impl EventReceiver {
    pub fn try_recv(&mut self) -> Result<Option<BusEvent>, EventBusError> {
        let mut queue = self
            .inner
            .queue
            .lock()
            .map_err(|_| EventBusError::Poisoned)?;

        Ok(queue.pop_front())
    }

    /// Waits until at least one event is queued and returns the whole batch.
    ///
    /// The future stays parked while the bus is empty, so an idle shell issues
    /// no wakeups at all instead of draining the queue on a timer.
    ///
    /// # Errors
    ///
    /// Returns [`EventBusError::Poisoned`] when a producer panicked while
    /// holding the queue lock.
    pub async fn recv(&mut self) -> Result<Vec<BusEvent>, EventBusError> {
        loop {
            let delivered = self.inner.delivered.notified();

            let batch = self.inner.drain()?;
            if !batch.is_empty() {
                return Ok(batch);
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

        bus.publish(snapshot(1)).expect("queue has room");
        bus.publish(BusEvent::PopupToggle).expect("queue has room");
        bus.publish(snapshot(2)).expect("queue has room");

        let batch = receiver.recv().await.expect("bus is healthy");

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
        bus.publish(BusEvent::Redraw).expect("queue has room");

        let batch = receiver.recv().await.expect("bus is healthy");

        assert_eq!(batch.len(), 1);
    }

    #[tokio::test]
    async fn recv_parks_until_a_producer_publishes() {
        let bus = bus();
        let mut receiver = bus.receiver();
        let sender = bus.sender();

        let waiting = tokio::spawn(async move { receiver.recv().await });

        assert!(!waiting.is_finished());

        sender
            .try_send(BusEvent::PopupToggle)
            .expect("queue has room");

        let batch = tokio::time::timeout(Duration::from_secs(1), waiting)
            .await
            .expect("receiver woke up")
            .expect("task did not panic")
            .expect("bus is healthy");

        assert!(matches!(batch.as_slice(), [BusEvent::PopupToggle]));
    }

    #[tokio::test]
    async fn recv_drains_the_whole_batch() {
        let bus = bus();
        let mut receiver = bus.receiver();
        bus.publish(BusEvent::Redraw).expect("queue has room");
        bus.publish(BusEvent::PopupToggle).expect("queue has room");

        let batch = receiver.recv().await.expect("bus is healthy");

        assert_eq!(batch.len(), 2);
        assert!(bus.drain().expect("bus is healthy").is_empty());
    }

    #[test]
    fn publish_rejects_events_once_the_queue_is_full() {
        let bus = EventBus::new(NonZeroUsize::new(1).expect("capacity is non zero"));
        bus.publish(BusEvent::Redraw).expect("queue has room");

        let err = bus
            .publish(BusEvent::PopupToggle)
            .expect_err("queue is full");

        assert_eq!(
            err,
            EventBusError::QueueFull {
                capacity: 1
            }
        );
    }
}
