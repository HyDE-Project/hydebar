use std::{any::TypeId, hash::Hash};

use hydebar_core::event_bus::{BusEvent, EventReceiver};
use iced::{
    Subscription,
    futures::stream::{BoxStream, StreamExt, unfold}
};
use iced_futures::subscription::{self, Recipe, from_recipe};
use log::error;

#[derive(Debug, Clone)]
pub struct BusFlushOutcome {
    events:    Vec<BusEvent>,
    had_error: bool
}

impl BusFlushOutcome {
    pub(super) fn with_events(events: Vec<BusEvent>, had_error: bool) -> Self {
        Self {
            events,
            had_error
        }
    }

    pub(super) fn had_error(&self) -> bool {
        self.had_error
    }

    pub(super) fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub(super) fn into_events(self) -> Vec<BusEvent> {
        self.events
    }
}

/// Subscription delivering module events as soon as a producer publishes them.
///
/// The stream parks on the bus instead of draining it on a timer, so an idle
/// shell performs no wakeups until a source has something to report.
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
        unfold(Some(self.receiver), |state| async move {
            let mut receiver = state?;

            match receiver.recv().await {
                Ok(events) => Some((BusFlushOutcome::with_events(events, false), Some(receiver))),
                Err(err) => {
                    error!("event bus is unusable, stopping the subscription: {err}");
                    Some((BusFlushOutcome::with_events(Vec::new(), true), None))
                }
            }
        })
        .boxed()
    }
}
