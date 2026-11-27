//! What each block of the canvas says, as plain text pairs.
//!
//! The canvas is a table of labels and values, so the reading is settled here
//! — before a single widget exists — and the drawing beside it only has to
//! place what this file already decided.
//!
//! Two rooms: [`machine`] reads the hardware off one system sample,
//! [`session`] reads the session the bar is running in. The data they answer
//! in — the [`Panel`], its [`Figure`]s and the row helper — stands in
//! [`panel`], below both the rooms and this root, so every room leans on it
//! directly and nothing leans back up through the root.

mod machine;
mod panel;
mod session;

pub(super) use machine::{
    cooling, cpu_temperature, graphics, memory, network, processor, storage, system
};
pub use panel::{Figure, Frame, Miniature, Panel};
pub(super) use session::{
    battery, desktop_menu, keyboard, link, notifications, own, playing, privacy, radio, screen,
    seat, session_idle, sound, submap, theme, tray, updates, wallpaper, weather, windows,
    workspaces
};
