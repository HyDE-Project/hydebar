//! Stepped ranges the size buttons of the settings menu walk.
//!
//! Each size is nudged one step at a time and clamped to the range the
//! bar can actually draw, so holding a button never runs a value off
//! into something unreadable.

use super::Settings;

/// Smallest bar height the menu will step down to, in pixels.
const MIN_HEIGHT: f32 = 16.0;
/// Largest bar height the menu will step up to, in pixels.
const MAX_HEIGHT: f32 = 96.0;
/// Height added or removed by one press, in pixels.
const HEIGHT_STEP: f32 = 2.0;

/// Smallest side padding the menu will step down to, in pixels.
///
/// Zero is a deliberate choice rather than a floor: a bar told to sit flush
/// with the screen edge is what a compositor without window gaps calls for.
const MIN_SIDE_PADDING: f32 = 0.0;
/// Largest side padding the menu will step up to, in pixels.
const MAX_SIDE_PADDING: f32 = 96.0;
/// Side padding added or removed by one press, in pixels.
const SIDE_PADDING_STEP: f32 = 1.0;

/// Smallest font size the menu will step down to, in pixels.
const MIN_FONT_SIZE: f32 = 6.0;
/// Largest font size the menu will step up to, in pixels.
const MAX_FONT_SIZE: f32 = 32.0;
/// Font size added or removed by one press, in pixels.
const FONT_SIZE_STEP: f32 = 1.0;

/// Opacity added or removed by one press.
const OPACITY_STEP: f32 = 0.05;

impl Settings {
    /// Height one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn height_below(current: f32) -> f32 {
        (current - HEIGHT_STEP).clamp(MIN_HEIGHT, MAX_HEIGHT)
    }

    /// Height one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn height_above(current: f32) -> f32 {
        (current + HEIGHT_STEP).clamp(MIN_HEIGHT, MAX_HEIGHT)
    }

    /// Side padding one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn side_padding_below(current: f32) -> f32 {
        (current - SIDE_PADDING_STEP).clamp(MIN_SIDE_PADDING, MAX_SIDE_PADDING)
    }

    /// Side padding one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn side_padding_above(current: f32) -> f32 {
        (current + SIDE_PADDING_STEP).clamp(MIN_SIDE_PADDING, MAX_SIDE_PADDING)
    }

    /// Font size one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn font_size_below(current: f32) -> f32 {
        (current - FONT_SIZE_STEP).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
    }

    /// Font size one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn font_size_above(current: f32) -> f32 {
        (current + FONT_SIZE_STEP).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
    }

    /// Opacity one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn opacity_below(current: f32) -> f32 {
        ((current - OPACITY_STEP) * 100.0).round() / 100.0
    }

    /// Opacity one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn opacity_above(current: f32) -> f32 {
        ((current + OPACITY_STEP) * 100.0).round() / 100.0
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    #[test]
    fn the_height_steps_stay_inside_the_supported_range() {
        assert_eq!(Settings::height_above(38.0), 40.0);
        assert_eq!(Settings::height_below(38.0), 36.0);
        assert_eq!(Settings::height_below(MIN_HEIGHT), MIN_HEIGHT);
        assert_eq!(Settings::height_above(MAX_HEIGHT), MAX_HEIGHT);
    }

    #[test]
    fn the_side_padding_steps_stay_inside_the_supported_range() {
        assert_eq!(Settings::side_padding_above(8.0), 9.0);
        assert_eq!(Settings::side_padding_below(8.0), 7.0);
        assert_eq!(
            Settings::side_padding_below(MIN_SIDE_PADDING),
            MIN_SIDE_PADDING
        );
        assert_eq!(
            Settings::side_padding_above(MAX_SIDE_PADDING),
            MAX_SIDE_PADDING
        );
    }

    #[test]
    fn the_font_size_steps_stay_inside_the_supported_range() {
        assert_eq!(Settings::font_size_above(10.0), 11.0);
        assert_eq!(Settings::font_size_below(10.0), 9.0);
        assert_eq!(Settings::font_size_below(MIN_FONT_SIZE), MIN_FONT_SIZE);
        assert_eq!(Settings::font_size_above(MAX_FONT_SIZE), MAX_FONT_SIZE);
    }

    #[test]
    fn the_opacity_steps_keep_two_decimals() {
        assert_eq!(Settings::opacity_above(0.8), 0.85);
        assert_eq!(Settings::opacity_below(0.8), 0.75);
    }
}
