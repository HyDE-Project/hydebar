//! Live indication of a desktop change the bar has asked for and cannot hurry.
//!
//! A HyDE theme switch rewrites the wallpaper, the palette and every generated
//! stylesheet, and takes seconds doing it. For all of those seconds the bar has
//! nothing to report except that it is still waiting, and a page that reported
//! it with a line of static text read exactly like a page that had missed the
//! press. What is drawn instead is a glyph that keeps moving: a still frame
//! cannot be told from a hung one, a moving frame can.
//!
//! The glyphs come from the icon font the bar bundles rather than from the text
//! font. A spinner built out of braille or box-drawing characters depends on
//! whatever the system font happens to cover, while every other icon on the bar
//! is already drawn from this font and is therefore certain to render.
//!
//! Nothing here reads a clock. The frame is state the module owns and advances
//! on a tick, so what the indicator shows is a pure function of how many ticks
//! have been delivered, and both the cycle and the pulse can be checked without
//! a frame clock, a compositor or a HyDE install.

use std::time::Duration;

/// Glyphs the indicator cycles through, in order.
///
/// The `circle-slice` series of the bundled icon font: a pie that fills one
/// eighth at a time and starts over. It reads as work in progress rather than
/// as a measured fraction of it, which is the honest thing to draw for a switch
/// whose remaining time the bar has no way of knowing.
const FRAMES: [&str; 8] = [
    "\u{f0a9e}",
    "\u{f0a9f}",
    "\u{f0aa0}",
    "\u{f0aa1}",
    "\u{f0aa2}",
    "\u{f0aa3}",
    "\u{f0aa4}",
    "\u{f0aa5}"
];

/// How long one frame stays on screen.
///
/// Fast enough to read as motion, slow enough that a whole switch costs a few
/// dozen redraws of a bar that is idle anyway. The bar asks the compositor for
/// a frame on this cadence only while a switch is running; the rest of the time
/// this constant costs nothing.
pub const FRAME_INTERVAL: Duration = Duration::from_millis(110);

/// Smallest share of its colour the pulsing mark is drawn at.
///
/// Above zero on purpose: a mark that faded out completely would blink rather
/// than pulse, and a blinking chip is indistinguishable from one being redrawn
/// wrongly.
const MIN_PULSE: f32 = 0.55;

/// Frame of the indicator drawn while the bar waits on the desktop.
///
/// Cheap to hold and cheap to copy: one index, advanced on a tick and read
/// while drawing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Spinner {
    /// Which of [`FRAMES`] is on screen.
    frame: usize
}

impl Spinner {
    /// Number of frames one full cycle takes.
    #[must_use]
    pub fn cycle() -> usize {
        FRAMES.len()
    }

    /// Moves the indicator on by one frame, starting the cycle over at the end.
    pub fn advance(&mut self) {
        self.frame = (self.frame + 1) % FRAMES.len();
    }

    /// Glyph this frame draws.
    #[must_use]
    pub fn glyph(self) -> &'static str {
        FRAMES[self.frame % FRAMES.len()]
    }

    /// Share of its colour a mark following this indicator is drawn at.
    ///
    /// Rises and falls over the cycle rather than sawing back to the start, so
    /// a chip tinted with it breathes instead of snapping dark once a cycle.
    #[must_use]
    pub fn pulse(self) -> f32 {
        let half = FRAMES.len() as f32 / 2.0;
        let position = self.frame as f32;
        let distance = (position - half).abs() / half;

        MIN_PULSE + (1.0 - MIN_PULSE) * distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_indicator_starts_at_the_first_frame() {
        assert_eq!(Spinner::default().glyph(), FRAMES[0]);
    }

    #[test]
    fn a_tick_moves_the_indicator_on() {
        let mut spinner = Spinner::default();
        spinner.advance();

        assert_eq!(spinner.glyph(), FRAMES[1]);
    }

    #[test]
    fn a_full_cycle_of_ticks_returns_to_the_first_frame() {
        let mut spinner = Spinner::default();

        for _ in 0..Spinner::cycle() {
            spinner.advance();
        }

        assert_eq!(spinner, Spinner::default());
    }

    #[test]
    fn every_frame_of_a_cycle_is_drawn_before_any_is_drawn_twice() {
        let mut spinner = Spinner::default();
        let mut seen = Vec::new();

        for _ in 0..Spinner::cycle() {
            seen.push(spinner.glyph());
            spinner.advance();
        }

        seen.sort_unstable();
        seen.dedup();

        assert_eq!(seen.len(), Spinner::cycle());
    }

    #[test]
    fn the_pulse_stays_inside_the_range_a_colour_can_be_scaled_by() {
        let mut spinner = Spinner::default();

        for _ in 0..Spinner::cycle() {
            let pulse = spinner.pulse();

            assert!((MIN_PULSE..=1.0).contains(&pulse), "{pulse}");
            spinner.advance();
        }
    }

    /// A mark that pulsed by the same amount on every frame would be a mark
    /// that does not pulse at all.
    #[test]
    fn the_pulse_moves_over_a_cycle() {
        let mut spinner = Spinner::default();
        let mut lowest = f32::MAX;
        let mut highest = f32::MIN;

        for _ in 0..Spinner::cycle() {
            lowest = lowest.min(spinner.pulse());
            highest = highest.max(spinner.pulse());
            spinner.advance();
        }

        assert!(highest - lowest > 0.2, "{lowest} to {highest}");
    }

    #[test]
    fn a_frame_lasts_long_enough_to_be_read_and_short_enough_to_be_motion() {
        assert!(FRAME_INTERVAL >= Duration::from_millis(60));
        assert!(FRAME_INTERVAL <= Duration::from_millis(200));
    }
}
