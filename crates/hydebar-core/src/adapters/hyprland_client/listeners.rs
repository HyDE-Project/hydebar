//! Background listeners bridging compositor events onto streams.

mod keyboard;
mod window;
mod workspace;

pub(crate) use keyboard::spawn_keyboard_listener;
pub(crate) use window::spawn_window_listener;
pub(crate) use workspace::spawn_workspace_listener;

const CHANNEL_CAPACITY: usize = 64;
