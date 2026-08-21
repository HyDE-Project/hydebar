//! Configuration of the desk: the bar's other form.
//!
//! The bar and the desk are one thing in two shapes. While a window is mapped
//! on a screen the bar is a strip along its edge; the moment the workspace is
//! cleared the very same modules, in the very same layout, come down off the
//! strip and stand over the wallpaper at a size the whole room can read. No
//! second set of readouts and no second arrangement: whatever the layout says
//! is on the bar is what the desk unfolds into.

use serde::Deserialize;

/// Largest magnification the desk may be drawn at.
///
/// A canvas drawn ten times the size of the strip would fit one module on the
/// screen; the ceiling keeps a mistyped number from hiding the layout.
const MAX_ZOOM: f32 = 6.0;

/// How the desk unfolds.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct DeskConfig {
    /// Whether the bar unfolds at all.
    pub enabled: bool,
    /// How much larger than the strip the modules are drawn on the canvas.
    pub zoom:    f32
}

impl DeskConfig {
    /// The magnification the canvas is actually drawn at.
    ///
    /// A number below one would draw the unfolded bar smaller than the strip
    /// it came off, which is never what was meant, and one far above the
    /// ceiling would leave a single module on the screen.
    #[must_use]
    pub const fn magnification(&self) -> f32 {
        if self.zoom.is_finite() {
            self.zoom.clamp(1.0, MAX_ZOOM)
        } else {
            default_zoom()
        }
    }
}

impl Default for DeskConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            zoom:    default_zoom()
        }
    }
}

/// Magnification the canvas is drawn at unless the configuration says
/// otherwise.
const fn default_zoom() -> f32 {
    2.0
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_desk_stays_folded_until_it_is_asked_for() {
        let config = DeskConfig::default();

        assert!(!config.enabled);
        assert!((config.magnification() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_named_magnification_is_taken_as_it_stands() {
        let config: DeskConfig = toml::from_str(
            r"
            enabled = true
            zoom = 3.5
            "
        )
        .expect("desk config");

        assert!(config.enabled);
        assert!((config.magnification() - 3.5).abs() < f32::EPSILON);
    }

    #[test]
    fn a_magnification_below_the_strip_or_past_the_ceiling_is_pulled_back() {
        for (named, drawn) in [(0.2, 1.0), (-4.0, 1.0), (40.0, MAX_ZOOM)] {
            let config = DeskConfig {
                enabled: true,
                zoom:    named
            };

            assert!(
                (config.magnification() - drawn).abs() < f32::EPSILON,
                "zoom {named} draws at {drawn}"
            );
        }
    }

    #[test]
    fn a_magnification_that_is_not_a_number_falls_back_to_the_stock_one() {
        let config = DeskConfig {
            enabled: true,
            zoom:    f32::NAN
        };

        assert!((config.magnification() - 2.0).abs() < f32::EPSILON);
    }
}
