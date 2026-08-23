//! The session the bar is running in: its power, its bells, its theme.
//!
//! Every reading here comes off a module the bar already keeps; nothing is
//! sampled twice and nothing is invented. A module that has not answered yet
//! contributes no rows, and a block with no rows is not drawn.
//!
//! Six rooms, by what the reading is about: [`power`] is the machine's own
//! supply and whether it is being held awake, [`notices`] is what the session
//! wants the user to know, [`desktop`] is the state of the desk itself,
//! [`devices`] is what the session speaks through, and [`windows`] is what
//! the compositor is holding, and [`own`] is a module the user wrote.

mod desktop;
mod devices;
mod notices;
mod own;
mod power;
mod windows;

pub use desktop::{desktop_menu, keyboard, seat, theme, tray, wallpaper, weather};
pub use devices::{link, radio, screen, sound};
pub use notices::{notifications, privacy, updates};
pub use own::own;
pub use power::{battery, session_idle};
pub use windows::{playing, submap, windows, workspaces};
