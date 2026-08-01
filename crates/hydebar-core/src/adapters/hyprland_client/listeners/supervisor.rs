//! Restart policy shared by the compositor event listeners.
//!
//! A listener is a long-lived socket connection that stays silent for as long
//! as nothing happens on the compositor, so silence carries no information: the
//! only observable failure is the connection itself ending. The supervisor
//! therefore waits on the listener rather than on a clock, stops the moment the
//! consumer drops its stream, and spaces reconnect attempts out with an
//! exponential backoff so a compositor that is gone cannot be hammered.

use std::time::Duration;

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
