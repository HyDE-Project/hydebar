//! The clock one unfolding runs on, and the two acts a block spends it in.
//!
//! Every block of the bar leaves at the same instant. A module has a lane of
//! its own down the screen and nothing standing in it, so there is nothing
//! for it to wait for; blocks setting off one after another is what reads as
//! the bar stalling, and it is slower into the bargain — the whole unfolding
//! then takes as long as the queue rather than as long as one flight.
//!
//! What a block does with the clock is in two acts rather than one: it
//! crosses the screen first and writes itself out second. Opening while still
//! travelling would have text sliding under the eye, and both acts leave
//! quickly and settle slowly, which is what everything arriving somewhere
//! does.

use std::time::Duration;

/// Share of the clock spent crossing, the rest spent opening.
const CROSSING: f32 = 0.55;

/// The even clock one screen's unfolding runs on.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Unfold {
    /// How far the unfolding has come, zero folded and one open.
    progress: f32
}

impl Unfold {
    /// How far the unfolding has come.
    #[must_use]
    pub const fn progress(self) -> f32 {
        self.progress
    }

    /// Reports whether anything of the unfolding is on screen.
    #[must_use]
    pub fn is_out(self) -> bool {
        self.progress > 0.0
    }

    /// Reports whether the unfolding still has travelling left to do.
    #[must_use]
    pub fn is_running(self) -> bool {
        self.progress > 0.0 && self.progress < 1.0
    }

    /// Folds the canvas away at once.
    ///
    /// Nothing travels home: the way back belongs to the strip, which has a
    /// window standing on it already and cannot wait for a flight to play.
    pub const fn fold(&mut self) {
        self.progress = 0.0;
    }

    /// Opens the canvas with nothing to play, as an unanimated bar asks.
    pub const fn open(&mut self) {
        self.progress = 1.0;
    }

    /// Advances the even clock by `elapsed` against a whole travel of `total`.
    ///
    /// Reports whether the clock is still running.
    pub fn advance(&mut self, elapsed: Duration, total: Duration) -> bool {
        if self.progress >= 1.0 {
            return false;
        }

        let total = total.as_secs_f32().max(f32::EPSILON);

        self.progress = (self.progress + elapsed.as_secs_f32() / total).min(1.0);

        self.progress < 1.0
    }
}

/// How far a block has crossed and how far it has opened, at `progress`.
///
/// One answer for every block of the bar, which is what it means for none of
/// them to wait: they are all in the same act at the same moment, and what
/// keeps them apart is the lane each one travels down.
#[must_use]
pub fn share(progress: f32) -> (f32, f32) {
    let progress = progress.clamp(0.0, 1.0);

    let travel = eased(progress / CROSSING);
    let bloom = eased((progress - CROSSING) / (1.0 - CROSSING));

    (travel, bloom)
}

/// The curve an act runs on: away quickly, into place slowly.
///
/// A cubic decelerate, which is what a thing arriving somewhere does
/// everywhere else on a desktop; the eye reads a linear arrival as a machine
/// stopping and a bouncing one as a toy.
fn eased(act: f32) -> f32 {
    let left = 1.0 - act.clamp(0.0, 1.0);

    left.mul_add(-(left * left), 1.0)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    #[test]
    fn an_even_clock_covers_the_whole_travel_in_the_time_it_was_given() {
        let mut clock = Unfold::default();
        let total = Duration::from_millis(500);
        let mut frames = 0;

        while clock.advance(Duration::from_millis(50), total) {
            frames += 1;
            assert!(frames < 100, "the clock settles");
        }

        assert_eq!(clock.progress(), 1.0);
        assert_eq!(frames, 9, "ten frames of fifty make five hundred");
    }

    #[test]
    fn nothing_of_the_bar_waits_for_anything_else() {
        for step in 1..400 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let progress = step as f32 / 400.0;

            assert!(
                share(progress).0 > 0.0,
                "every block is under way at {progress:.3}"
            );
        }
    }

    #[test]
    fn a_block_crosses_before_it_opens() {
        assert_eq!(share(0.0), (0.0, 0.0));
        assert_eq!(share(1.0), (1.0, 1.0));

        for step in 0..=200 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let progress = step as f32 / 200.0;
            let (travel, bloom) = share(progress);

            assert!(
                bloom == 0.0 || travel >= 1.0,
                "nothing opens before it has arrived"
            );
        }
    }

    #[test]
    fn the_front_moves_on_every_frame_of_the_clock() {
        let mut moving = share(0.0);

        for step in 1..=400 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let progress = step as f32 / 400.0;
            let next = share(progress);

            assert!(
                next.0 > moving.0 || next.1 > moving.1 || next == (1.0, 1.0),
                "something is still on the move at {progress:.3}"
            );

            moving = next;
        }
    }

    #[test]
    fn the_curve_leaves_quickly_and_settles_slowly() {
        assert_eq!(eased(0.0), 0.0);
        assert_eq!(eased(1.0), 1.0);
        assert!(eased(0.5) > 0.5, "half the time is more than half the way");
        assert!(
            eased(0.9) - eased(0.8) < eased(0.2) - eased(0.1),
            "the last stretch is slower than the first"
        );
    }
}
