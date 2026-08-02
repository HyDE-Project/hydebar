#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
use flexi_logger::LogSpecification;

mod centerbox;
mod views;

pub mod app;

pub use app::{App, Message};

/// Builds the logger specification for a configured level.
///
/// A level that does not parse falls back to `info` instead of failing: the
/// same string arrives from a live configuration reload, where a panic would
/// take the whole bar down over a typo in one key.
#[must_use]
pub fn get_log_spec(log_level: &str) -> LogSpecification {
    LogSpecification::env_or_parse(log_level).unwrap_or_else(|err| {
        log::warn!("unreadable log level `{log_level}`, staying on `info`: {err}");
        LogSpecification::info()
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_valid_level_parses_into_a_spec() {
        let spec = get_log_spec("debug");

        assert_eq!(
            spec.module_filters()[0].level_filter,
            log::LevelFilter::Debug
        );
    }

    #[test]
    fn an_unreadable_level_falls_back_to_info_instead_of_panicking() {
        let spec = get_log_spec("hydebar=very-loud");

        assert_eq!(
            spec.module_filters()[0].level_filter,
            log::LevelFilter::Info
        );
    }
}
