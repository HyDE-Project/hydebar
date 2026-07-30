//! Icon catalogue, the overrides applied to it and the widgets rendering it.
//!
//! Split three ways on purpose: the catalogue is data, the theme is the state
//! built from the configuration, and the widget layer is the only part that
//! knows about the renderer.

mod catalog;
mod optical;
mod theme;
mod widget;

#[cfg(test)]
mod tests;

pub use catalog::Icons;
pub use theme::IconTheme;
pub use widget::{icon, icon_raw, icon_raw_sized};
