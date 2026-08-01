//! Hyprland port contract, split by concern.
//!
//! The error type lives in [`error`], the snapshots, metadata and selectors in
//! [`data`], the event enums in [`events`] and the port trait itself, with the
//! stream alias its subscriptions hand back, in [`port`].

mod data;
mod error;
mod events;
mod port;

pub use data::{
    HyprlandClientInfo, HyprlandKeyboardState, HyprlandMonitorInfo, HyprlandMonitorSelector,
    HyprlandWindowInfo, HyprlandWorkspaceInfo, HyprlandWorkspaceSelector,
    HyprlandWorkspaceSnapshot
};
pub use error::HyprlandError;
pub use events::{HyprlandKeyboardEvent, HyprlandWindowEvent, HyprlandWorkspaceEvent};
pub use port::{HyprlandEventStream, HyprlandPort};
