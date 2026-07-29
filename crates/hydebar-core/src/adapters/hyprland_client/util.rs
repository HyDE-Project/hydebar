use std::time::Duration;

/// Compute the delay to wait before retrying an operation using linear backoff.
///
/// The returned duration is `base_backoff * attempt` with saturating
/// multiplication.
///
/// # Examples
///
/// ```ignore
/// use std::time::Duration;
/// use hydebar_core::adapters::hyprland_client::util::calculate_retry_delay;
///
/// let delay = calculate_retry_delay(Duration::from_millis(100), 3);
/// assert_eq!(delay, Duration::from_millis(300));
/// ```
pub(crate) fn calculate_retry_delay(base_backoff: Duration, attempt: u8) -> Duration {
    if attempt == 0 {
        return Duration::ZERO;
    }

    base_backoff.saturating_mul(u32::from(attempt))
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;

    use super::calculate_retry_delay;

    #[test]
    fn retry_delay_uses_linear_backoff() {
        assert_eq!(
            calculate_retry_delay(Duration::from_millis(50), 0),
            Duration::ZERO
        );
        assert_eq!(
            calculate_retry_delay(Duration::from_millis(50), 1),
            Duration::from_millis(50)
        );
        assert_eq!(
            calculate_retry_delay(Duration::from_millis(50), 2),
            Duration::from_millis(100)
        );
    }

}
