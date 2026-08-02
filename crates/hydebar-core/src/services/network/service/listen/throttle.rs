//! Rate limit on the kernel link reads.

use std::time::{Duration, Instant};

/// Keeps the kernel link reads to one per window however often signals tick.
#[derive(Debug, Default)]
pub(super) struct LinkThrottle {
    last: Option<Instant>
}

impl LinkThrottle {
    /// Shortest gap between two throttled link reads.
    pub(super) const WINDOW: Duration = Duration::from_secs(10);

    /// Whether a read may go out at `now`, claiming the window when it may.
    pub(super) fn admits(&mut self, now: Instant) -> bool {
        if self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) < Self::WINDOW)
        {
            return false;
        }

        self.last = Some(now);

        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::LinkThrottle;

    #[test]
    fn the_throttle_window_reopens_after_it_passes() {
        let mut throttle = LinkThrottle::default();
        let start = Instant::now();

        assert!(throttle.admits(start));
        assert!(!throttle.admits(start + Duration::from_secs(5)));
        assert!(throttle.admits(start + LinkThrottle::WINDOW));
    }
}
