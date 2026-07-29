//! Output management façade, re-exporting the collection state and helpers.

mod config;
pub mod scaling;
mod state;
mod wayland;

pub use scaling::{AutoMetrics, metrics as auto_metrics};
pub use state::{HasOutput, Outputs};
