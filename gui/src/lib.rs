//! The bar itself: the state machine every surface renders from, and the
//! composition that turns a configuration into the three sections of a bar.
//!
//! Modules live in the core crate and know nothing of this one. What happens
//! here is wiring: registering a module while the layout hosts it, dispatching
//! a press to whoever answers it, and assembling the elements the modules
//! return into the islands the compositor is handed.

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![warn(missing_docs)]
use tracing_subscriber::{EnvFilter, Registry, reload};

/// Shared filter handle, re-applicable at runtime.
pub type Logger = reload::Handle<EnvFilter, Registry>;

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
///
/// `hydebar*` is a group quieted or loudened apart from the noisy renderers;
/// in debug builds the bar's own crates always trace down to `debug`.
#[must_use]
pub fn log_spec(log_level: &str) -> EnvFilter {
    let level = match log_level
        .parse::<log::LevelFilter>()
        .unwrap_or(log::LevelFilter::Info)
    {
        log::LevelFilter::Off => "off",
        log::LevelFilter::Error => "error",
        log::LevelFilter::Warn => "warn",
        log::LevelFilter::Info => "info",
        log::LevelFilter::Debug => "debug",
        log::LevelFilter::Trace => "trace"
    };

    let hydebar = if cfg!(debug_assertions) {
        "debug"
    } else {
        level
    };

    EnvFilter::try_new(format!("{level},hydebar={hydebar}"))
        .unwrap_or_else(|_| EnvFilter::new("info"))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_valid_level_parses_into_a_spec() {
        assert_eq!(
            log_spec("debug").max_level_hint(),
            Some(tracing_subscriber::filter::LevelFilter::DEBUG)
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn the_bar_keeps_its_own_tracing_in_a_debug_build() {
        let spec = log_spec("warn").to_string();

        assert!(spec.contains("hydebar=debug"), "spec was: {spec}");
        assert!(spec.contains("warn"), "spec was: {spec}");
    }

    #[test]
    fn an_unreadable_level_falls_back_to_info_instead_of_panicking() {
        assert!(log_spec("hydebar=very-loud").to_string().contains("info"));
    }
}
