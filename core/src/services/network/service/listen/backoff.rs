//! Reconnect pacing after failed connection attempts.

use std::time::Duration;

/// Shortest pause between two connection attempts.
const RECONNECT_MIN_DELAY: Duration = Duration::from_secs(1);

/// Longest pause between two connection attempts.
const RECONNECT_MAX_DELAY: Duration = Duration::from_secs(64);

/// Pause before reconnecting after `failures` consecutive failed attempts.
///
/// A machine running neither `NetworkManager` nor iwd fails the D-Bus
/// connection immediately, so a fixed one second retry becomes a permanent one
/// hertz wakeup that logs an error every time. Doubling the delay up to a
/// minute keeps a transient failure recovering within a second while an absent
/// backend settles into a cost the idle bar does not notice.
pub(super) fn reconnect_delay(failures: u32) -> Duration {
    let shift = failures.saturating_sub(1).min(u32::BITS - 1);

    RECONNECT_MIN_DELAY
        .saturating_mul(1u32 << shift)
        .min(RECONNECT_MAX_DELAY)
}

#[cfg(test)]
mod tests {
    use super::{RECONNECT_MAX_DELAY, RECONNECT_MIN_DELAY, reconnect_delay};

    #[test]
    fn the_first_failure_retries_after_the_shortest_delay() {
        assert_eq!(reconnect_delay(1), RECONNECT_MIN_DELAY);
    }

    #[test]
    fn consecutive_failures_double_the_delay() {
        assert_eq!(reconnect_delay(2), RECONNECT_MIN_DELAY * 2);
        assert_eq!(reconnect_delay(3), RECONNECT_MIN_DELAY * 4);
        assert_eq!(reconnect_delay(4), RECONNECT_MIN_DELAY * 8);
    }

    #[test]
    fn a_backend_that_never_appears_stops_at_the_longest_delay() {
        assert_eq!(reconnect_delay(100), RECONNECT_MAX_DELAY);
        assert_eq!(reconnect_delay(u32::MAX), RECONNECT_MAX_DELAY);
    }
}
