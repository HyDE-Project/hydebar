use std::{future::Future, pin::Pin};

use iced::{
    Subscription, Task,
    futures::{SinkExt, channel::mpsc::Sender}
};

pub(crate) mod bus;

pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod hyprland_notify;
pub mod idle_inhibitor;
pub mod mpris;
pub mod network;
pub mod notifications;
pub mod privacy;
pub mod tray;
pub mod upower;

#[derive(Debug, Clone)]
pub enum ServiceEvent<S: ReadOnlyService> {
    Init(S),
    Update(S::UpdateEvent),
    Error(S::Error)
}

/// Smallest pause between reconnect attempts of a failed service.
pub(crate) const RECONNECT_MIN_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

/// Largest pause a repeatedly failing service settles into.
pub(crate) const RECONNECT_MAX_DELAY: std::time::Duration = std::time::Duration::from_mins(1);

/// Delay before the next attempt after `failures` consecutive failures.
///
/// One law for every service on purpose: a backend that is merely restarting
/// recovers within a second, an absent one settles into a wakeup a minute the
/// idle bar does not notice — and no service is allowed to spin or to stay
/// dead for the session.
pub(crate) fn reconnect_delay(failures: u32) -> std::time::Duration {
    let shift = failures.saturating_sub(1).min(u32::BITS - 1);

    RECONNECT_MIN_DELAY
        .saturating_mul(1u32 << shift)
        .min(RECONNECT_MAX_DELAY)
}

pub trait Service: ReadOnlyService {
    type Command;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>>;
}

pub trait ReadOnlyService: Sized {
    type UpdateEvent;
    type Error: Clone;

    fn update(&mut self, event: Self::UpdateEvent);

    fn subscribe() -> Subscription<ServiceEvent<Self>>;
}

pub trait ServiceEventPublisher<S: ReadOnlyService> {
    type SendFuture<'a>: Future<Output = ()> + Send + 'a
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<S>) -> Self::SendFuture<'_>;
}

impl<S> ServiceEventPublisher<S> for Sender<ServiceEvent<S>>
where
    S: ReadOnlyService + 'static + Send,
    S::UpdateEvent: Send,
    S::Error: Send
{
    type SendFuture<'a>
        = Pin<Box<dyn Future<Output = ()> + Send + 'a>>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<S>) -> Self::SendFuture<'_> {
        Box::pin(async move {
            let _ = SinkExt::send(self, event).await;
        })
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{RECONNECT_MAX_DELAY, RECONNECT_MIN_DELAY, reconnect_delay};

    #[test]
    fn the_first_attempt_waits_the_minimum_delay() {
        assert_eq!(reconnect_delay(0), RECONNECT_MIN_DELAY);
        assert_eq!(reconnect_delay(1), RECONNECT_MIN_DELAY);
    }

    #[test]
    fn each_further_failure_doubles_the_delay() {
        let mut previous = reconnect_delay(1);

        for failures in 2..=7 {
            let current = reconnect_delay(failures);
            assert_eq!(current, previous * 2, "failures = {failures}");
            previous = current;
        }
    }

    #[test]
    fn the_delay_never_exceeds_the_maximum() {
        assert_eq!(reconnect_delay(8), RECONNECT_MAX_DELAY);

        for failures in 8..=64 {
            assert_eq!(
                reconnect_delay(failures),
                RECONNECT_MAX_DELAY,
                "failures = {failures}"
            );
        }
    }

    #[test]
    fn a_huge_failure_count_neither_panics_nor_overflows() {
        assert_eq!(reconnect_delay(u32::MAX), RECONNECT_MAX_DELAY);
        assert_eq!(reconnect_delay(u32::MAX - 1), RECONNECT_MAX_DELAY);
        assert_eq!(reconnect_delay(u32::BITS), RECONNECT_MAX_DELAY);
    }
}
