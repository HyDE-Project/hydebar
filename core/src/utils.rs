use std::{sync::LazyLock, time::Duration};

pub mod hyde_shell;
pub mod launcher;
pub mod process_group;

/// How long one HTTP request may take before it is abandoned.
///
/// Every fetch loop in the bar awaits its request before ticking again, so a
/// request with no deadline hangs that loop for as long as the connection
/// dangles — the weather simply stops updating.
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);

/// The one HTTP client every module shares.
///
/// A bare convenience request builds a fresh client — TLS setup and a
/// connection pool — per call and carries no timeout. Building it once gives
/// every caller the deadline and lets requests to the same host reuse their
/// connection.
static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .connect_timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default()
});

/// The shared HTTP client.
#[must_use]
pub fn http_client() -> &'static reqwest::Client {
    &HTTP_CLIENT
}

#[derive(Debug)]
pub enum IndicatorState {
    Normal,
    Success,
    Warning,
    Danger
}

#[must_use]
pub fn format_duration(duration: &Duration) -> String {
    let h = duration.as_secs() / 60 / 60;
    let m = duration.as_secs() / 60 % 60;
    if h > 0 {
        format!("{h}h {m:>2}m")
    } else {
        format!("{m:>2}m")
    }
}

#[must_use]
pub fn truncate_text(value: &str, max_length: u32) -> String {
    let length = value.chars().count();

    if length > max_length as usize {
        let split = max_length as usize / 2;
        let first_part = value.chars().take(split).collect::<String>();
        let last_part = value.chars().skip(length - split).collect::<String>();
        format!("{first_part}...{last_part}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_long_multi_byte_title_keeps_both_ends() {
        let truncated = truncate_text("Привет мир как дела", 5);

        assert_eq!(truncated, "Пр...ла");
    }

    #[test]
    fn every_small_max_survives_multi_byte_input() {
        for max in 0..8 {
            let _ = truncate_text("Привет мир как дела", max);
        }
    }

    #[test]
    fn ascii_input_is_truncated_as_before() {
        assert_eq!(truncate_text("hello world", 6), "hel...rld");
    }

    #[test]
    fn a_string_no_longer_than_the_limit_is_untouched() {
        assert_eq!(truncate_text("Привет", 6), "Привет");
        assert_eq!(truncate_text("short", 10), "short");
    }
}
