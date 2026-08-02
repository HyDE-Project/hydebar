//! Magnification decided before the renderer starts.
//!
//! The renderer takes its default text size once, when it is built, and never
//! looks at it again. A bar that wants every label to grow with the screen has
//! to know the screen before that moment, so the compositor is asked directly
//! rather than waiting for the first output event.

use std::process::Command;

use hydebar_core::outputs::auto_metrics;
use log::debug;

/// Geometry of the screen the bar will land on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenGeometry {
    /// Resolution in physical pixels.
    pub(crate) pixels:   (f32, f32),
    /// Physical size in millimetres, zero when the screen does not report it.
    pub(crate) physical: (f32, f32),
    /// Scale the compositor already applies to the surface.
    pub(crate) scale:    f32
}

impl ScreenGeometry {
    /// Magnification this screen calls for.
    pub(crate) fn magnification(self) -> f32 {
        auto_metrics(self.pixels.0, self.pixels.1, self.scale, self.physical).scale
    }
}

/// Reads the geometry of the focused screen from the compositor.
///
/// Returns nothing when the compositor cannot be reached or answers something
/// unexpected, in which case the bar simply keeps the configured sizes.
pub fn focused_screen() -> Option<ScreenGeometry> {
    let output = Command::new("hyprctl")
        .args(["-j", "monitors"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    parse_focused(&String::from_utf8_lossy(&output.stdout))
}

/// Picks the focused screen out of the compositor answer.
fn parse_focused(json: &str) -> Option<ScreenGeometry> {
    let monitors: serde_json::Value = serde_json::from_str(json).ok()?;
    let monitors = monitors.as_array()?;

    let monitor = monitors
        .iter()
        .find(|monitor| monitor["focused"].as_bool().unwrap_or(false))
        .or_else(|| monitors.first())?;

    #[expect(
        clippy::cast_possible_truncation,
        reason = "monitor geometry fits well within f32 precision"
    )]
    let number = |key: &str| monitor[key].as_f64().unwrap_or(0.0) as f32;

    let geometry = ScreenGeometry {
        pixels:   (number("width"), number("height")),
        physical: (number("physicalWidth"), number("physicalHeight")),
        scale:    number("scale").max(1.0)
    };

    debug!("focused screen reports {geometry:?}");

    Some(geometry)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;

    const ANSWER: &str = r#"[
        {"name":"HDMI-A-1","focused":true,"width":3840,"height":2160,
         "physicalWidth":1650,"physicalHeight":930,"scale":1.0},
        {"name":"eDP-1","focused":false,"width":1920,"height":1080,
         "physicalWidth":344,"physicalHeight":193,"scale":1.0}
    ]"#;

    #[test]
    fn the_focused_screen_is_the_one_read() {
        let geometry = parse_focused(ANSWER).expect("a focused screen");

        assert_eq!(geometry.pixels, (3840.0, 2160.0));
        assert_eq!(geometry.physical, (1650.0, 930.0));
        assert_eq!(geometry.scale, 1.0);
    }

    #[test]
    fn a_four_megapixel_screen_asks_for_a_double_bar() {
        let geometry = parse_focused(ANSWER).expect("a focused screen");

        assert_eq!(geometry.magnification(), 2.0);
    }

    #[test]
    fn a_laptop_panel_asks_for_nothing() {
        let answer = r#"[{"name":"eDP-1","focused":true,"width":1920,"height":1080,
            "physicalWidth":344,"physicalHeight":193,"scale":1.0}]"#;

        let geometry = parse_focused(answer).expect("a focused screen");

        assert_eq!(geometry.magnification(), 1.0);
    }

    #[test]
    fn no_focused_screen_falls_back_to_the_first() {
        let answer = r#"[{"name":"DP-1","focused":false,"width":2560,"height":1440,
            "physicalWidth":600,"physicalHeight":340,"scale":1.0}]"#;

        assert_eq!(
            parse_focused(answer).expect("a screen").pixels,
            (2560.0, 1440.0)
        );
    }

    #[test]
    fn an_unexpected_answer_is_reported_as_nothing() {
        assert!(parse_focused("not json").is_none());
        assert!(parse_focused("[]").is_none());
    }
}
