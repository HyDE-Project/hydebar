//! The session the bar is running in: its power, its bells, its theme.
//!
//! Every reading here comes off a module the bar already keeps; nothing is
//! sampled twice and nothing is invented. A module that has not answered yet
//! contributes no rows, and a block with no rows is not drawn.
//!
//! Three rooms, by what the reading is about: [`power`] is the machine's own
//! supply and whether it is being held awake, [`notices`] is what the session
//! wants the user to know, and [`desktop`] is the state of the desk itself.

mod desktop;
mod notices;
mod power;

pub use desktop::{keyboard, theme, tray, weather};
pub use notices::{notifications, privacy, updates};
pub use power::{battery, session_idle};
