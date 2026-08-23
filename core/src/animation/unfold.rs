//! The clock one unfolding runs on, and the two acts a block spends it in.
//!
//! Every block of the bar leaves at the same instant. A module has a lane of
//! its own down the screen and nothing standing in it, so there is nothing
//! for it to wait for; blocks setting off one after another is what reads as
//! the bar stalling, and it is slower into the bargain — the whole unfolding
//! then takes as long as the queue rather than as long as one flight.
//!
//! What a block does with the clock it does the moment it can. It falls out
//! of the row it shared, closes in along its own lane while it is still
//! falling, and comes straight down into its place; it writes itself out from
//! the instant it is there. And the near blocks are on their level first: the
//! way is shorter and the speed is the same, so the top of a column has
//! finished opening while
//! the bottom of it is still coming down. The crossing settles slowly and
//! sets off just as slowly — a block is watched leaving its place on the
//! strip as much as arriving at its place on the canvas — and the opening,
//! which arrives from nowhere, only settles slowly.

use std::time::Duration;

/// Share of the clock the block with the furthest to go spends crossing.
const CROSSING: f32 = 0.55;

/// How much of the clock one breath of light lasts.
///
/// Short: the light marks an instant — the setting off, the settling — and an
/// instant that is held is a state rather than a moment. Long enough that the
/// bar's own frames can draw it rising and falling rather than blinking.
const FLARE: f32 = 0.16;

/// How long a block takes to write itself out, once it is down.
///
/// The same stretch for every block, which is what makes the near ones finish
/// before the far ones start. Timing the opening to end with the clock
/// instead had every block, near or far, finish writing on the same frame —
/// so however early a block landed, the whole canvas still settled at once.
///
/// Measured back from the end of the clock off the furthest block's landing,
/// so that block is written out on the very frame the unfolding is over.
fn opening() -> f32 {
    1.0 - landed(1.0)
}

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
    let arrival = landed(reach);

    let travel = crossed(progress / crossing);
    let bloom = eased((progress - arrival) / opening());

    (travel, bloom)
}

/// How brightly a block is lit at `progress`, on a journey `reach` long.
///
/// Two breaths of light and nothing between them: one where the block leaves
/// the place it held, one where it settles into the place it was going to.
/// What the eye is given is the two ends of the journey — a thing coming
/// loose and a thing coming to rest — and lighting the whole crossing would
/// say only that the block is lit.
///
/// Both are out well before the clock is, so a canvas standing open carries
/// no glow at all: this is the light of something moving, and nothing here
/// is moving any more.
#[must_use]
pub fn flare(progress: f32, reach: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);

    let setting_off = breath(progress / FLARE);
    let settling = breath((progress - landed(reach)) / FLARE + 0.5);

    setting_off.max(settling)
}

/// One breath of light over its own stretch: dark, bright, dark again.
///
/// Rounded rather than triangular. A light that came up and went out at one
/// pace read as a lamp being switched, and what is wanted is the glow of a
/// thing gathering itself and letting go.
fn breath(act: f32) -> f32 {
    if !(0.0..=1.0).contains(&act) {
        return 0.0;
    }

    let away = 2.0f32.mul_add(act, -1.0);

    away.mul_add(-away, 1.0)
}

/// When a block whose journey is `reach` long is down on its level.
///
/// The share of the clock its drop takes, which is what anything timed off a
/// block landing needs: the strip's own background goes out behind its
/// islands rather than before them, so it has to be told when each of them
/// has landed.
///
/// Read off the crossing curve rather than assumed from the share of the
/// journey the drop takes: a curve that is not a straight line puts the
/// landing somewhere else in the clock than the distance alone says, and a
/// background timed off the wrong instant goes out from under a block that
/// is still on its way down.
#[must_use]
pub fn landed(reach: f32) -> f32 {
    CROSSING * reach.clamp(0.05, 1.0) * descended()
}

/// How far into its crossing a block is when its drop is over.
///
/// The crossing curve read backwards from [`DESCENT`]: the one place the
/// answer is worked out, so nothing has to guess where on the clock a block
/// touches down.
///
/// Closed in on by halving rather than solved. A cubic carrying a slope of
/// its own at both ends has no inverse worth writing down, and the two dozen
/// halvings it takes to land inside a millionth cost less than the shadow
/// they time.
///
/// [`DESCENT`]: crate::components::flip::DESCENT
fn descended() -> f32 {
    let mut before = 0.0f32;
    let mut after = 1.0f32;

    for _ in 0..24 {
        let between = before.midpoint(after);

        if crossed(between) < crate::components::flip::DESCENT {
            before = between;
        } else {
            after = between;
        }
    }

    before.midpoint(after)
}

/// The curve a journey between two places runs on: away at once, along at its
/// own pace, into place slowly.
///
/// Two things the crossing has to be, and only one curve is both.
///
/// It cannot spend itself on the first instant. Run on the decelerate curve
/// the acts that arrive from nowhere use, a block was out of the strip's band
/// a frame or two after setting off and crept the rest of the way, which
/// reads as the strip dropping its modules rather than carrying them.
///
/// It cannot hold still either. The strip hands a block to the canvas on the
/// very frame the clock starts, and the canvas draws it in the shape it will
/// stand in — its own pill rather than the one it shared. A curve that leaves
/// slowly, as a symmetric one does, leaves that change sitting on the strip
/// for a tenth of a second before anything moves, so the bar is seen changing
/// before the transition rather than during it.
///
/// So: the pace of a straight line at the setting off, which is what carries
/// the handover, easing to nothing at the arrival, which is what everything
/// coming to rest does.
fn crossed(act: f32) -> f32 {
    let act = act.clamp(0.0, 1.0);

    (act * act).mul_add(1.0 - act, act)
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
            let arrival = landed(reach);

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

    #[test]
    fn a_crossing_sets_off_at_once_and_settles_slowly() {
        assert_eq!(crossed(0.0), 0.0);
        assert_eq!(crossed(1.0), 1.0);

        assert!(
            crossed(0.1) >= 0.1,
            "a block is behind a straight line at the setting off"
        );
        assert!(
            crossed(0.1) < eased(0.1) / 2.0,
            "the first tenth of a crossing spends the speed of a whole arrival"
        );
        assert!(
            crossed(1.0) - crossed(0.9) < crossed(0.1) - crossed(0.0),
            "the last stretch is not slower than the first"
        );
    }

    #[test]
    fn a_block_is_already_moving_on_the_frame_the_strip_lets_go_of_it() {
        const FRAME: f32 = 0.027;

        for reach in [0.2_f32, 0.5, 1.0] {
            assert!(
                share(FRAME, reach).0 > 0.02,
                "a block reaching {reach} stands still on the frame the canvas takes it, \
                 so the bar is seen changing before the transition"
            );
        }
    }

    #[test]
    fn a_block_is_lit_where_it_comes_loose_and_where_it_comes_to_rest() {
        assert_eq!(flare(0.0, 1.0), 0.0, "a block at rest on the strip is dark");
        assert_eq!(flare(1.0, 1.0), 0.0, "a canvas standing open is dark");

        assert!(flare(FLARE / 2.0, 1.0) > 0.9, "the setting off is not lit");
        assert!(flare(landed(1.0), 1.0) > 0.9, "the settling is not lit");
        assert_eq!(
            flare(landed(1.0).midpoint(FLARE), 1.0),
            0.0,
            "the crossing between the two ends is lit"
        );
    }

    #[test]
    fn the_light_of_a_journey_is_out_before_the_clock_is() {
        for reach in [0.2_f32, 0.5, 1.0] {
            let settled = landed(reach) + FLARE;

            for step in 0..=100 {
                #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
                let progress = (1.0 - settled).mul_add(step as f32 / 100.0, settled);

                assert_eq!(
                    flare(progress, reach),
                    0.0,
                    "a block reaching {reach} is still lit at {progress:.3}"
                );
            }
        }
    }

    #[test]
    fn a_landing_is_the_instant_the_drop_is_over() {
        for reach in [0.2_f32, 0.5, 1.0] {
            let down = landed(reach);

            assert!(
                (share(down, reach).0 - crate::components::flip::DESCENT).abs() < 1e-4,
                "the drop of a block reaching {reach} is not over when it is said to be"
            );
            assert!(
                share(down - 0.01, reach).0 < crate::components::flip::DESCENT,
                "a block reaching {reach} is down before it is said to be"
            );
        }
    }
}
