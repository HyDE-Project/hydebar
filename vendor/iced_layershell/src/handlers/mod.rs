//! Wayland protocol handlers for `iced_layershell`.
//!
//! Each submodule implements handlers for a specific Wayland protocol:
//! - `compositor`: `wl_compositor` surface management
//! - `layer_shell`: `wlr_layer_shell` layer surfaces
//! - `seat`: `wl_seat` capability management
//! - `keyboard`: `wl_keyboard` input
//! - `pointer`: `wl_pointer` input
//! - `output`: `wl_output` monitor tracking
//! - `touch`: `wl_touch` input
//! - `data_device`: clipboard and drag-and-drop

pub mod compositor;
pub mod data_device;
pub mod keyboard;
pub mod layer_shell;
pub mod output;
pub mod pointer;
pub mod seat;
pub mod touch;
