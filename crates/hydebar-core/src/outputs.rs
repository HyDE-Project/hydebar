//! Output management façade, re-exporting the collection state and helpers.

mod blur;
mod config;
pub mod scaling;
mod state;
mod wayland;

pub use scaling::{AutoMetrics, metrics as auto_metrics};
pub use state::{HasOutput, Outputs};

/// How long after a theme change the compositor may still reload.
///
/// A HyDE switch repaints the bar early — the palette lands well before the
/// scripts finish — and ends, seconds later, with a compositor reload that
/// forgets every dynamically stated rule. A single restatement raced that
/// reload and lost.
const RELOAD_TAIL: std::time::Duration = std::time::Duration::from_secs(5);

/// States the blur rules to the compositor again, off the caller's thread.
///
/// The rules are handed over dynamically and the compositor forgets them on
/// every configuration reload — and a HyDE theme switch ends in exactly such a
/// reload. The bar re-states them whenever the desktop theme moved, which is
/// the one wipe it can observe, and once more after [`RELOAD_TAIL`] so the
/// statement lands on the far side of the reload that follows the switch. A
/// bar that only asked at startup keeps its blur until the first theme switch
/// and looks broken ever after.
pub fn restate_blur() {
    std::thread::spawn(|| {
        blur::request();
        std::thread::sleep(RELOAD_TAIL);
        blur::request();
    });
}
