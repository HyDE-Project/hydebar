//! Adapter speaking to the Hyprland compositor through `hyprland-rs`.
//!
//! The client handle lives in [`client`], the port operations in [`port`],
//! the translation of raw compositor records in [`snapshot`], the dispatcher
//! dialects in [`dispatch`], the retry policy in [`sync_ops`] and the event
//! listeners in [`listeners`].

mod client;
mod config;
mod dispatch;
mod listeners;
mod port;
mod snapshot;
mod sync_ops;
mod util;

pub use self::{client::HyprlandClient, config::HyprlandClientConfig};
