//! One conversation with the outside world each, and what they publish.
//!
//! A service owns a bus connection, a socket or a device and nothing else: it
//! listens, it restates what it heard as an event of its own, and it hands
//! that to whoever subscribed. Modules read services; services never read
//! modules.

use std::{future::Future, pin::Pin};

use iced::{
    Subscription, Task,
    futures::{SinkExt, channel::mpsc::Sender}
};

pub(crate) mod bus;

/// The sound server: outputs, inputs and their volumes.
pub mod audio;
/// The bluetooth adapter and the devices paired with it.
pub mod bluetooth;
/// The backlight.
pub mod brightness;
/// Notices sent through the compositor.
pub mod hyprland_notify;
/// Keeping the screen awake.
pub mod idle_inhibitor;
/// Whatever is playing, through the media bus.
pub mod mpris;
/// Links, wireless networks and VPNs.
pub mod network;
/// The notification bus, when the bar serves it.
pub mod notifications;
/// Whether the microphone, camera or screen is being read.
pub mod privacy;
/// The icons applications register with the system tray.
pub mod tray;
/// The battery and the power profile.
pub mod upower;

/// What a service publishes over its lifetime.
#[derive(Debug, Clone)]
pub enum ServiceEvent<S: ReadOnlyService> {
    /// The service connected, and this is what it found.
    Init(S),
    /// Something changed.
    Update(S::UpdateEvent),
    /// The conversation failed.
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

/// A service that can be told to do something, not only listened to.
pub trait Service: ReadOnlyService {
    /// What this service accepts being told.
    type Command;

    /// Tells the service to do something, and says what follows.
    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>>;
}

/// A service that is only ever listened to.
pub trait ReadOnlyService: Sized {
    /// What this service publishes when something changes.
    type UpdateEvent;

    /// How this service says the conversation failed.
    type Error: Clone;

    /// Folds one published change into the state held here.
    fn update(&mut self, event: Self::UpdateEvent);

    /// Starts the conversation and hands back what it publishes.
    fn subscribe() -> Subscription<ServiceEvent<Self>>;
}

/// Where a service driver hands what it has to publish.
///
/// A trait rather than the channel itself, so a driver can be exercised
/// against something that records what it was told instead of a live bus.
pub trait ServiceEventPublisher<S: ReadOnlyService> {
    /// The future one publication completes through.
    type SendFuture<'a>: Future<Output = ()> + Send + 'a
    where
        Self: 'a;

    /// Publishes one event.
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
