//! Metrics derived from the screen the bar is drawn on.
//!
//! A bar stated in fixed pixels is readable on the screen it was tuned for and
//! nowhere else: the same 10 pixel text is comfortable on a laptop panel and a
//! squint on a four megapixel screen. These rules turn what the compositor
//! reports about an output into a text size and a bar height, so a fresh
//! install is readable before anything is configured.

/// Diagonal of the screen the built in sizes were tuned for, in pixels.
///
/// A full high definition screen: the sizes of the reference waybar theme are
/// comfortable there, and every other screen is expressed relative to it.
const REFERENCE_DIAGONAL_PX: f32 = 2202.9072;

/// Never shrink below the reference: a smaller screen keeps the tuned sizes
/// rather than becoming unreadable.
const MIN_FACTOR: f32 = 1.0;

/// Never grow past this, however large the screen reports itself to be.
const MAX_FACTOR: f32 = 3.0;

/// Diagonal of the screen the built in sizes were tuned for, in millimetres.
///
/// A twenty four inch desktop monitor at arm's length.
const REFERENCE_DIAGONAL_MM: f32 = 610.0;

/// Ceiling of the contribution the physical size may make.
///
/// A television is further away, not proportionally further: doubling is enough
/// to keep it readable across a room, and letting the physical diagonal run to
/// the overall ceiling on its own turns a bar into a banner.
const MAX_PHYSICAL_FACTOR: f32 = 2.0;

/// How much the bar is magnified on a given screen.
///
/// A single number rather than a size per element on purpose: the surface is
/// scaled as a whole, so text, glyphs, paddings and radii all grow together and
/// the bar cannot lose its proportions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoMetrics {
    /// Factor the whole surface is drawn at.
    pub scale: f32
}

/// Factor the reference sizes are multiplied by on a screen of `width` by
/// `height` logical pixels.
///
/// The diagonal is used rather than either side so an ultrawide screen is not
/// mistaken for a screen twice as tall.
#[must_use]
pub fn scale_factor(width: f32, height: f32) -> f32 {
    if width <= 0.0 || height <= 0.0 {
        return MIN_FACTOR;
    }

    let diagonal = width.hypot(height);

    (diagonal / REFERENCE_DIAGONAL_PX).clamp(MIN_FACTOR, MAX_FACTOR)
}

/// Factor a screen of `width` by `height` millimetres calls for.
///
/// People sit further from a larger screen, so a bar that keeps its size in
/// millimetres shrinks away on a television. Growing with the physical diagonal
/// keeps roughly the same angular size at every viewing distance.
#[must_use]
pub fn physical_factor(width_mm: f32, height_mm: f32) -> f32 {
    if width_mm <= 0.0 || height_mm <= 0.0 {
        return MIN_FACTOR;
    }

    (width_mm.hypot(height_mm) / REFERENCE_DIAGONAL_MM).clamp(MIN_FACTOR, MAX_PHYSICAL_FACTOR)
}

/// Magnification for a screen of `width` by `height` physical pixels reported
/// at `compositor_scale`, `physical` millimetres across.
///
/// The compositor scale is divided out: a screen the compositor already doubles
/// hands the bar half as many logical pixels, and the bar must not double them
/// a second time.
///
/// The result is rounded to quarters, so a screen that reports itself slightly
/// differently after a mode change does not nudge the whole bar by a pixel.
#[must_use]
pub fn metrics(
    width: f32,
    height: f32,
    compositor_scale: f32,
    physical: (f32, f32)
) -> AutoMetrics {
    let scale = if compositor_scale > 0.0 {
        compositor_scale
    } else {
        1.0
    };

    let by_pixels = scale_factor(width / scale, height / scale);
    let by_size = physical_factor(physical.0, physical.1);
    let factor = by_pixels.max(by_size).clamp(MIN_FACTOR, MAX_FACTOR);

    AutoMetrics {
        scale: (factor * 4.0).round() / 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A twenty four inch monitor, the screen the sizes were tuned on.
    const REFERENCE_MM: (f32, f32) = (531.0, 299.0);

    /// A seventy five inch television.
    const TELEVISION_MM: (f32, f32) = (1650.0, 930.0);

    #[test]
    fn the_reference_screen_is_not_magnified() {
        assert_eq!(metrics(1920.0, 1080.0, 1.0, REFERENCE_MM).scale, 1.0);
    }

    #[test]
    fn a_dense_desktop_screen_is_magnified_by_its_pixels() {
        assert_eq!(metrics(3840.0, 2160.0, 1.0, REFERENCE_MM).scale, 2.0);
    }

    #[test]
    fn a_low_resolution_television_is_magnified_by_its_size() {
        let scale = metrics(1920.0, 1080.0, 1.0, TELEVISION_MM).scale;

        assert!(scale > 1.0);
        assert_eq!(
            scale,
            (physical_factor(TELEVISION_MM.0, TELEVISION_MM.1) * 4.0).round() / 4.0
        );
    }

    #[test]
    fn a_large_and_dense_television_takes_the_stronger_signal() {
        let scale = metrics(3840.0, 2160.0, 1.0, TELEVISION_MM).scale;

        assert!(scale >= metrics(3840.0, 2160.0, 1.0, REFERENCE_MM).scale);
    }

    #[test]
    fn a_screen_the_compositor_already_scales_is_not_scaled_twice() {
        assert_eq!(metrics(3840.0, 2160.0, 2.0, REFERENCE_MM).scale, 1.0);
    }

    #[test]
    fn a_smaller_screen_is_left_alone() {
        assert_eq!(metrics(1366.0, 768.0, 1.0, REFERENCE_MM).scale, 1.0);
    }

    #[test]
    fn the_magnification_lands_on_a_quarter() {
        let scale = metrics(2560.0, 1440.0, 1.0, REFERENCE_MM).scale;

        assert_eq!(scale * 4.0, (scale * 4.0).round());
    }

    #[test]
    fn width_alone_does_not_decide_the_factor() {
        assert!(scale_factor(3440.0, 1440.0) < scale_factor(3440.0, 2160.0));
    }

    #[test]
    fn an_enormous_screen_stops_at_the_ceiling() {
        assert_eq!(
            metrics(15360.0, 8640.0, 1.0, TELEVISION_MM).scale,
            MAX_FACTOR
        );
    }

    #[test]
    fn a_screen_that_reports_no_size_is_not_magnified() {
        assert_eq!(scale_factor(0.0, 0.0), MIN_FACTOR);
        assert_eq!(physical_factor(0.0, 0.0), MIN_FACTOR);
        assert_eq!(metrics(0.0, 0.0, 0.0, (0.0, 0.0)).scale, MIN_FACTOR);
    }
}
