//! The clock one unfolding runs on, and the two acts a block spends it in.
//!
//! Every block of the bar leaves at the same instant. A module has a lane of
//! its own down the screen and nothing standing in it, so there is nothing
//! for it to wait for; blocks setting off one after another is what reads as
//! the bar stalling, and it is slower into the bargain — the whole unfolding
//! then takes as long as the queue rather than as long as one flight.
//!
//! What a block does with the clock it does the moment it can. It drops to
//! its own level and writes itself out from the instant it is on it, while
//! the last of its journey — the move along its own lane — still runs under
//! it. And the near blocks are on their level first: the way is shorter and
//! the speed is the same, so the top of a column is open while the bottom of
//! it is still coming down. Both acts leave quickly and settle slowly, which
//! is what everything arriving somewhere does.

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
/// `reach` is how far this block has to go against the block that goes
/// furthest, one being the furthest of them. Every block sets off at the same
/// instant and they all travel at the same speed, so a block with half the
/// way to go is there in half the time — and opens there and then, rather
/// than standing on its place waiting for the far ones to land.
#[must_use]
pub fn share(progress: f32, reach: f32) -> (f32, f32) {
    let progress = progress.clamp(0.0, 1.0);
    let reach = reach.clamp(0.05, 1.0);

    let crossing = CROSSING * reach;
    let arrival = crossing * crate::components::flip::DESCENT;

    let travel = eased(progress / crossing);
    let bloom = eased((progress - arrival) / (1.0 - arrival));

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

            for reach in [0.2_f32, 0.5, 1.0] {
                assert!(
                    share(progress, reach).0 > 0.0,
                    "every block is under way at {progress:.3}"
                );
            }
        }
    }

    #[test]
    fn the_nearer_block_arrives_first_and_opens_first() {
        for step in 1..400 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let progress = step as f32 / 400.0;

            let near = share(progress, 0.3);
            let far = share(progress, 1.0);

            assert!(
                near.0 >= far.0 && near.1 >= far.1,
                "at {progress:.3} the near block is behind the far one: {near:?} against {far:?}"
            );
        }

        assert!(
            share(0.2, 0.3).1 > 0.0,
            "the near block is open while the far one is still coming down"
        );
        assert_eq!(share(0.2, 1.0).1, 0.0);
    }

    #[test]
    fn a_block_drops_to_its_level_before_it_opens() {
        assert_eq!(share(0.0, 1.0), (0.0, 0.0));
        assert_eq!(share(1.0, 1.0), (1.0, 1.0));

        for reach in [0.2_f32, 0.5, 1.0] {
            let arrival = CROSSING * reach * crate::components::flip::DESCENT;

            for step in 0..=200 {
                #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
                let progress = step as f32 / 200.0;
                let (_, bloom) = share(progress, reach);

                assert!(
                    bloom == 0.0 || progress >= arrival,
                    "nothing opens before it is down at {progress:.3}"
                );
            }
        }
    }

    #[test]
    fn a_block_never_stands_still_between_arriving_and_opening() {
        for step in 0..=400 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let progress = step as f32 / 400.0;

            if progress <= 0.0 || progress >= 1.0 {
                continue;
            }

            let (travel, bloom) = share(progress, 1.0);

            assert!(
                travel < 1.0 || bloom > 0.0,
                "at {progress:.3} the journey is over and nothing has begun to open"
            );
        }
    }

    #[test]
    fn the_front_moves_on_every_frame_of_the_clock() {
        let mut moving = share(0.0, 1.0);

        for step in 1..=400 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let progress = step as f32 / 400.0;
            let next = share(progress, 1.0);

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
