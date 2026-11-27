//! The bar itself: the state machine every surface renders from, and the
//! composition that turns a configuration into the three sections of a bar.
//!
//! Modules live in the core crate and know nothing of this one. What happens
//! here is wiring: registering a module while the layout hosts it, dispatching
//! a press to whoever answers it, and assembling the elements the modules
//! return into the islands the compositor is handed.

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![warn(missing_docs)]
use flexi_logger::LogSpecification;

mod centerbox;
mod views;

/// The bar's state machine: what it holds, how it answers, what it draws.
pub mod app;

pub use app::{App, Message};

/// Builds the logger specification for a configured level.
///
/// The configured level governs what the user asked to see. A debug build
/// keeps tracing the bar's own crates down to `debug` on top of it, so a
/// `cargo run` session can follow every frame and every surface decision
/// even when the configuration quiets the rest of the world down to `warn`.
///
/// A level that does not parse falls back to `info` instead of failing: the
/// same string arrives from a live configuration reload, where a panic would
/// take the whole bar down over a typo in one key.
#[must_use]
pub fn log_spec(log_level: &str) -> LogSpecification {
    let level = log_level.parse().unwrap_or(log::LevelFilter::Info);

    if cfg!(debug_assertions) {
        LogSpecification::builder()
            .default(level)
            .module("hydebar", log::LevelFilter::Debug)
            .build()
    } else {
        LogSpecification::builder().default(level).build()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_valid_level_parses_into_a_spec() {
        let spec = log_spec("debug");

        assert_eq!(spec.level_for_module(None), Some(log::LevelFilter::Debug));
    }

    #[test]
    fn the_bar_keeps_its_own_tracing_in_a_debug_build() {
        let spec = log_spec("warn");

        assert_eq!(
            spec.level_for_module(Some("hydebar")),
            Some(log::LevelFilter::Debug)
        );
        assert_eq!(spec.level_for_module(None), Some(log::LevelFilter::Warn));
    }

    #[test]
    fn an_unreadable_level_falls_back_to_info_instead_of_panicking() {
        let spec = log_spec("hydebar=very-loud");

        assert_eq!(spec.level_for_module(None), Some(log::LevelFilter::Info));
    }
}
