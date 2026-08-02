//! One compositor connection serving every event subscriber.
//!
//! The window, workspace and keyboard listeners — and the blur guard beside
//! them — used to hold a socket each, so every compositor event was parsed
//! four times by four supervisors. One multiplexed listener now owns the one
//! socket, and every subscriber taps a broadcast channel instead of the
//! compositor.
//!
//! The singleton and its supervisor live in [`singleton`], the handler
//! registrations in [`wiring`] and the subscription surface in [`taps`].

mod singleton;
mod taps;
mod wiring;

pub use taps::{config_reloads, keyboard_events, window_events, workspace_events};
