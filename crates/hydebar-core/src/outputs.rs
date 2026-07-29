//! Output management façade, re-exporting the collection state and helpers.

mod blur;
mod config;
pub mod scaling;
mod state;
mod wayland;

pub use scaling::{AutoMetrics, metrics as auto_metrics};
pub use state::{HasOutput, Outputs};

/// Width of the strip the notification popups own, in the units a menu is laid
/// out in.
///
/// The strip is stated in physical pixels because that is what the compositor
/// is told; a menu is laid out before the renderer scales it, so the figure has
/// to be divided back out or the reservation would be as many times too wide as
/// the bar is magnified.
#[must_use]
pub fn notifications_strip_width(scale_factor: f64) -> f32 {
    if scale_factor <= 0.0 {
        return wayland::NOTIFICATIONS_WIDTH as f32;
    }

    (f64::from(wayland::NOTIFICATIONS_WIDTH) / scale_factor) as f32
}
