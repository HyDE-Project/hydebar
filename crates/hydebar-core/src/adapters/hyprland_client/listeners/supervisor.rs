//! Restart policy shared by the compositor event listeners.
//!
//! A listener is a long-lived socket connection that stays silent for as long
//! as nothing happens on the compositor, so silence carries no information: the
//! only observable failure is the connection itself ending. The supervisor
//! therefore waits on the listener rather than on a clock, stops the moment the
//! consumer drops its stream, and spaces reconnect attempts out with an
//! exponential backoff so a compositor that is gone cannot be hammered.

use std::time::Duration;

use hydebar_proto::ports::hyprland::HyprlandError;
use hyprland::event_listener::AsyncEventListener;
use log::warn;
use tokio::{
    sync::mpsc,
    time::{Instant, sleep}
};

use super::super::HyprlandClient;

/// Longest a reconnect may be delayed, however many attempts failed before.
const MAX_RESTART_DELAY: Duration = Duration::from_secs(30);

/// Delay preceding the `attempt`-th reconnect.
///
/// The delay doubles per consecutive failure and then flattens at
/// [`MAX_RESTART_DELAY`], so a compositor that never comes back costs one
/// connection attempt every thirty seconds instead of a spin.
pub(crate) fn restart_delay(base: Duration, attempt: u32) -> Duration {
    if base.is_zero() || attempt == 0 {
        return Duration::ZERO;
    }

    let factor = 1_u32.checked_shl(attempt - 1).unwrap_or(u32::MAX);

    base.saturating_mul(factor).min(MAX_RESTART_DELAY)
}

/// Keeps a compositor listener connected until the consumer goes away.
///
/// `build` produces a freshly wired listener for every attempt; it receives the
/// sender so the event handlers it installs can publish onto the stream.
///
/// A listener that stayed connected for at least `stability_window` counts as
/// healthy, which resets the backoff: a compositor restart hours into a session
/// should reconnect promptly rather than inherit the delay of an unrelated
/// failure at startup.
pub(crate) async fn supervise<T, B>(
    operation: &'static str,
    tx: mpsc::Sender<Result<T, HyprlandError>>,
    base_delay: Duration,
    stability_window: Duration,
    build: B
) where
    B: Fn(&mpsc::Sender<Result<T, HyprlandError>>) -> AsyncEventListener
{
    let mut attempt = 0_u32;

    loop {
        let mut listener = build(&tx);
        let started = Instant::now();

        let outcome = tokio::select! {
            biased;
            () = tx.closed() => return,
            result = listener.start_listener_async() => result
        };

        if started.elapsed() >= stability_window {
            attempt = 0;
        }

        match outcome {
            Ok(()) => {
                warn!(
                    target: "hydebar::hyprland",
                    "listener connection closed, reconnecting (operation={operation})"
                );
            }
            Err(err) => {
                if tx
                    .send(Err(HyprlandClient::backend_error(operation, err)))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        }

        attempt = attempt.saturating_add(1);

        tokio::select! {
            biased;
            () = tx.closed() => return,
            () = sleep(restart_delay(base_delay, attempt)) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{MAX_RESTART_DELAY, restart_delay};

    #[test]
    fn a_zero_base_never_delays() {
        assert_eq!(restart_delay(Duration::ZERO, 5), Duration::ZERO);
    }

    #[test]
    fn the_first_attempt_waits_the_base_delay() {
        assert_eq!(
            restart_delay(Duration::from_millis(250), 1),
            Duration::from_millis(250)
        );
    }

    #[test]
    fn consecutive_attempts_double_the_delay() {
        let base = Duration::from_millis(250);

        assert_eq!(restart_delay(base, 2), Duration::from_millis(500));
        assert_eq!(restart_delay(base, 3), Duration::from_secs(1));
        assert_eq!(restart_delay(base, 4), Duration::from_secs(2));
    }

    #[test]
    fn the_delay_flattens_at_the_cap() {
        let base = Duration::from_millis(250);

        assert_eq!(restart_delay(base, 20), MAX_RESTART_DELAY);
        assert_eq!(restart_delay(base, 200), MAX_RESTART_DELAY);
    }
}
