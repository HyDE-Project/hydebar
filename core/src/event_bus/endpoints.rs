//! The handles producers and the consumer hold on the shared queue.
//!
//! The bus itself, its cloneable sender and the receiver all share one inner
//! queue; the receiver's future stays parked while the bus is empty, so an
//! idle shell issues no wakeups at all instead of draining the queue on a
//! timer.

use std::{num::NonZeroUsize, sync::Arc};

use super::queue::{BusEvent, EventBusInner};

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
    ///
    /// The first event of a burst is delivered the moment it lands, without a
    /// grace period: bursts are tamed at the queue itself — a snapshot
    /// replaces its stale twin, a duplicate redraw folds into the tail — and
    /// whatever arrives while a batch is being handled is picked up whole by
    /// the next call. A collection window here would tax every user click
    /// with latency to save work the queue already saves.
    pub async fn recv(&mut self) -> Vec<BusEvent> {
        loop {
            let delivered = self.inner.delivered.notified();

            let batch = self.inner.drain();
            if !batch.is_empty() {
                return batch;
            }

            delivered.await;
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{num::NonZeroUsize, time::Duration};

    use super::*;
    use crate::event_bus::ModuleEvent;

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
}
