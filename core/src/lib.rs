//! What the bar is made of: its modules, the services behind them and the
//! bus that carries what they have to say.
//!
//! A module owns its data, its update logic and its rendering; a service owns
//! one conversation with the outside world — a bus, a socket, a device — and
//! publishes what it hears. Nothing here composes a bar: that is the interface
//! crate's work, and this crate never calls into it.

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![warn(missing_docs)]

/// Default height of the main status bar in logical pixels.
pub const HEIGHT: f64 = 34.;

pub mod adapters;
pub mod animation;
/// The module the user is looking at, and the two clocks that follow it.
pub mod attention;
/// Widgets and glyphs every module draws with.
pub mod components;
pub mod config;
/// Event bus primitives for communicating UI updates across the core.
pub mod event_bus;
pub mod format_cycle;
pub mod menu;
/// What a module is handed in order to start its work.
pub mod module_context;
/// The bar entries themselves.
pub mod modules;
pub mod notifications_popup;
pub mod outputs;
/// The dialog that asks for a network secret.
pub mod password_dialog;
pub mod position_button;
pub mod services;
/// How everything the bar draws is coloured and sized.
pub mod style;
/// Fixtures the crate's own tests and the crates above it both build on.
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
pub mod tooltip;
/// Odds and ends: launching, supervising, reaping.
pub mod utils;

pub use module_context::{ModuleContext, ModuleEventSender};
