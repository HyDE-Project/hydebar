//! Menus opened from the bar: what they are, how they fade and where they land.

mod dismiss_area;
mod kind;
mod size;
mod state;
mod wrapper;

#[cfg(test)]
mod tests;

pub use dismiss_area::{DismissArea, dismiss_area};
pub use kind::MenuType;
pub use size::{MenuMetrics, MenuSize};
pub use state::Menu;
pub use wrapper::{MenuLayout, PADDING_EM as MENU_PADDING_EM, menu_wrapper};
