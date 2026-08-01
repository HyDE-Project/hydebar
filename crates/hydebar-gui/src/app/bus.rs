use std::{any::TypeId, hash::Hash};

use hydebar_core::event_bus::{BusEvent, EventReceiver};
use iced::{
    Subscription,
    futures::stream::{BoxStream, StreamExt, unfold}
};
use iced_futures::subscription::{self, Recipe, from_recipe};

/// A batch of events drained from the bus in one wakeup.
#[derive(Debug, Clone)]
pub struct BusFlushOutcome {
    events: Vec<BusEvent>
}

impl BusFlushOutcome {
    pub(super) const fn with_events(events: Vec<BusEvent>) -> Self {
        Self {
            events
        }
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub(super) fn into_events(self) -> Vec<BusEvent> {
        self.events
    }
}

/// Subscription delivering module events as soon as a producer publishes them.
///
/// The stream parks on the bus instead of draining it on a timer, so an idle
/// shell performs no wakeups until a source has something to report. The bus
/// itself is infallible, so the subscription lives as long as the bar does.
pub(super) fn subscription(receiver: EventReceiver) -> Subscription<BusFlushOutcome> {
    from_recipe(BusWatcher {
        receiver
    })
}

struct BusWatcher {
    receiver: EventReceiver
}

impl Recipe for BusWatcher {
    type Output = BusFlushOutcome;

    fn hash(&self, state: &mut subscription::Hasher) {
        TypeId::of::<Self>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream
    ) -> BoxStream<'static, Self::Output> {
        unfold(self.receiver, |mut receiver| async move {
            let events = receiver.recv().await;

            Some((BusFlushOutcome::with_events(events), receiver))
        })
        .boxed()
    }
}
