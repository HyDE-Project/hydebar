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

/// Text size that is comfortable on the reference screen, in pixels.
const REFERENCE_FONT_SIZE: f32 = 10.0;

/// Bar height that is comfortable on the reference screen, in pixels.
const REFERENCE_HEIGHT: f32 = 38.0;

/// Never shrink below the reference: a smaller screen keeps the tuned sizes
/// rather than becoming unreadable.
const MIN_FACTOR: f32 = 1.0;

/// Never grow past this, however large the screen reports itself to be.
const MAX_FACTOR: f32 = 3.0;

/// Sizes the bar takes on a given screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoMetrics {
    /// Text size, in logical pixels.
    pub font_size: f32,
    /// Bar height, in logical pixels.
    pub height:    f32
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

/// Sizes for a screen of `width` by `height` physical pixels reported at
/// `compositor_scale`.
///
/// The compositor scale is divided out: a screen the compositor already doubles
/// hands the bar half as many logical pixels, and the bar must not double the
/// sizes a second time.
#[must_use]
pub fn metrics(width: f32, height: f32, compositor_scale: f32) -> AutoMetrics {
    let scale = if compositor_scale > 0.0 {
        compositor_scale
    } else {
        1.0
    };

    let factor = scale_factor(width / scale, height / scale);

    AutoMetrics {
        font_size: (REFERENCE_FONT_SIZE * factor).round(),
        height:    (REFERENCE_HEIGHT * factor).round()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_screen_keeps_the_reference_sizes() {
        let metrics = metrics(1920.0, 1080.0, 1.0);

        assert_eq!(metrics.font_size, REFERENCE_FONT_SIZE);
        assert_eq!(metrics.height, REFERENCE_HEIGHT);
    }

    #[test]
    fn a_four_times_larger_screen_doubles_the_sizes() {
        let metrics = metrics(3840.0, 2160.0, 1.0);

        assert_eq!(metrics.font_size, REFERENCE_FONT_SIZE * 2.0);
        assert_eq!(metrics.height, REFERENCE_HEIGHT * 2.0);
    }

    #[test]
    fn a_screen_the_compositor_already_scales_is_not_scaled_twice() {
        let doubled = metrics(3840.0, 2160.0, 2.0);

        assert_eq!(doubled.font_size, REFERENCE_FONT_SIZE);
        assert_eq!(doubled.height, REFERENCE_HEIGHT);
    }

    #[test]
    fn a_smaller_screen_keeps_the_reference_sizes() {
        let metrics = metrics(1366.0, 768.0, 1.0);

        assert_eq!(metrics.font_size, REFERENCE_FONT_SIZE);
        assert_eq!(metrics.height, REFERENCE_HEIGHT);
    }

    #[test]
    fn width_alone_does_not_decide_the_factor() {
        let wide_and_short = scale_factor(3440.0, 1440.0);
        let wide_and_tall = scale_factor(3440.0, 2160.0);

        assert!(wide_and_short < wide_and_tall);
    }

    #[test]
    fn an_enormous_screen_stops_at_the_ceiling() {
        let metrics = metrics(15360.0, 8640.0, 1.0);

        assert_eq!(
            metrics.font_size,
            (REFERENCE_FONT_SIZE * MAX_FACTOR).round()
        );
        assert_eq!(metrics.height, (REFERENCE_HEIGHT * MAX_FACTOR).round());
    }

    #[test]
    fn a_screen_of_no_size_falls_back_to_the_reference() {
        assert_eq!(scale_factor(0.0, 0.0), MIN_FACTOR);
        assert_eq!(metrics(0.0, 0.0, 0.0).font_size, REFERENCE_FONT_SIZE);
    }
}
