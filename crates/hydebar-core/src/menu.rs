//! Menus opened from the bar: what they are, how they fade and where they land.

mod kind;
mod size;
mod state;
mod wrapper;

#[cfg(test)]
mod tests;

pub use kind::MenuType;
pub use size::MenuSize;
pub use state::Menu;
pub use wrapper::{MenuLayout, menu_wrapper};
