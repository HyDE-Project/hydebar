//! The clock one unfolding runs on, and the share each block takes of it.
//!
//! A row of blocks leaving a strip is a sequence, not a single move, and the
//! two want different clocks. The sequence wants an even one: every block
//! deserves the same slice of the travel, and a spring driving the whole
//! sequence gives the last blocks the tail of its own curve, where it barely
//! moves — which reads as the last ones stalling. Each block on its own wants
//! the opposite: a curve that leaves quickly and settles slowly, the way
//! everything that arrives somewhere does.
//!
//! So the whole travel runs on an even clock and every block eases within its
//! own share of it, and the shares overlap deeply: a block sets off while the
//! one before it is still crossing, so the column comes down as one movement
//! rather than as a queue taking turns.

use std::time::Duration;

/// Share of one block's slice spent crossing, the rest spent opening.
const CROSSING: f32 = 0.55;

/// How much of a block's slice the next block waits before it sets off.
///
/// Kept far below [`CROSSING`] on purpose: a block leaves while the one
/// before it is still crossing, so what the eye follows is a single body of
/// movement with the blocks fanning out inside it. Near one the blocks would
/// hand the screen to one another instead, and a sequence of short flights
/// separated by handovers is what reads as every block waiting its turn — the
/// whole thing takes longer and none of it flows.
const LEAD: f32 = 0.14;

/// The even clock one screen's unfolding runs on.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Unfold {
    /// How far the whole sequence has come, zero folded and one open.
    progress: f32
}

impl Unfold {
    /// How far the whole sequence has come.
    #[must_use]
    pub const fn progress(self) -> f32 {
        self.progress
    }

    /// Reports whether anything of the sequence is on screen.
    #[must_use]
    pub fn is_out(self) -> bool {
        self.progress > 0.0
    }

    /// Reports whether the sequence still has travelling left to do.
    #[must_use]
    pub fn is_running(self) -> bool {
        self.progress > 0.0 && self.progress < 1.0
    }

    /// Folds the sequence away at once.
    ///
    /// Nothing travels home: the way back belongs to the strip, which has a
    /// window standing on it already and cannot wait for a sequence to play.
    pub const fn fold(&mut self) {
        self.progress = 0.0;
    }

    /// Opens the sequence with nothing to play, as an unanimated bar asks.
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

/// How far the block at `place` has crossed and how far it has opened.
///
/// `place` is where the block stands in the sequence, zero leading and one
/// trailing, and `blocks` is how many share the travel.
#[must_use]
pub fn share(progress: f32, place: f32, blocks: usize) -> (f32, f32) {
    let own = eased(local(progress, place, blocks));

    if own <= CROSSING {
        (own / CROSSING, 0.0)
    } else {
        (1.0, (own - CROSSING) / (1.0 - CROSSING))
    }
}

/// How much longer a sequence of `blocks` takes than a single block does.
///
/// The overlap is what a clock is set by: give one block the time it needs to
/// cross and open, multiply by this, and every block of the sequence has that
/// same time to itself however many of them there are. Timing the whole by
/// the count instead would hand each block a share of a fixed budget, and the
/// fuller the bar the more each block would hurry.
#[must_use]
pub fn stretch(blocks: usize) -> f32 {
    1.0 / slice(blocks)
}

/// The raw share of the travel one block has used, before it is eased.
fn local(progress: f32, place: f32, blocks: usize) -> f32 {
    let slice = slice(blocks);
    let start = place.clamp(0.0, 1.0) * (1.0 - slice);

    ((progress.clamp(0.0, 1.0) - start) / slice).clamp(0.0, 1.0)
}

/// The share of the whole travel one block's own move takes.
///
/// Every block gets the same slice, and the slices are spread over the whole
/// travel with [`LEAD`] of a slice between one start and the next.
#[expect(
    clippy::cast_precision_loss,
    reason = "a layout holds a handful of blocks, far below any precision limit"
)]
fn slice(blocks: usize) -> f32 {
    if blocks < 2 {
        return 1.0;
    }

    (1.0 / ((blocks - 1) as f32).mul_add(LEAD, 1.0)).clamp(0.05, 1.0)
}

/// The curve one block travels on: away quickly, into place slowly.
///
/// A cubic decelerate, which is what a thing arriving somewhere does
/// everywhere else on a desktop; the eye reads a linear arrival as a machine
/// stopping and a bouncing one as a toy.
fn eased(local: f32) -> f32 {
    let left = 1.0 - local.clamp(0.0, 1.0);

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
    fn every_block_is_given_the_same_slice_of_the_travel() {
        for blocks in 2..8usize {
            let slice = slice(blocks);
            let starts: Vec<f32> = (0..blocks)
                .map(|index| {
                    #[expect(clippy::cast_precision_loss, reason = "a handful of blocks")]
                    let place = index as f32 / (blocks - 1) as f32;

                    place * (1.0 - slice)
                })
                .collect();

            for pair in starts.windows(2) {
                let gap = pair[1] - pair[0];

                assert!(
                    slice.mul_add(-LEAD, gap).abs() < 1e-4,
                    "{blocks} blocks: the gap between starts is one lead"
                );
            }
        }
    }

    #[test]
    fn the_front_never_stops_between_one_block_and_the_next() {
        for blocks in 2..8usize {
            for step in 0..=400 {
                #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
                let progress = step as f32 / 400.0;

                if progress <= 0.0 || progress >= 1.0 {
                    continue;
                }

                let moving = (0..blocks).any(|index| {
                    #[expect(clippy::cast_precision_loss, reason = "a handful of blocks")]
                    let place = index as f32 / (blocks - 1) as f32;
                    let own = local(progress, place, blocks);

                    own > 0.0 && own < 1.0
                });

                assert!(moving, "{blocks} blocks: nothing moves at {progress:.3}");
            }
        }
    }

    #[test]
    fn a_block_sets_off_while_the_one_before_it_is_still_crossing() {
        for blocks in 2..8usize {
            let slice = slice(blocks);

            #[expect(clippy::cast_precision_loss, reason = "a handful of blocks")]
            let second = (1.0 / (blocks - 1) as f32) * (1.0 - slice);
            let (travel, _) = share(second, 0.0, blocks);

            assert!(
                travel > 0.0 && travel < 1.0,
                "{blocks} blocks: the first is still crossing when the second sets off"
            );
        }
    }

    #[test]
    fn a_sequence_takes_its_own_stretch_of_one_block() {
        assert_eq!(stretch(1), 1.0);
        assert!((stretch(2) - (1.0 + LEAD)).abs() < 1e-4);
        assert!(
            stretch(12) < 3.0,
            "a full bar is not a dozen flights end to end"
        );
    }

    #[test]
    fn a_block_crosses_before_it_opens() {
        let (travel, bloom) = share(0.0, 0.0, 4);
        assert_eq!((travel, bloom), (0.0, 0.0));

        let (travel, bloom) = share(1.0, 1.0, 4);
        assert_eq!((travel, bloom), (1.0, 1.0));

        for step in 0..=200 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let progress = step as f32 / 200.0;
            let (travel, bloom) = share(progress, 0.0, 4);

            assert!(
                bloom == 0.0 || travel >= 1.0,
                "nothing opens before it has arrived"
            );
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
    fn a_lone_block_takes_the_whole_travel() {
        assert_eq!(slice(1), 1.0);
        assert_eq!(local(0.5, 0.0, 1), 0.5);
    }
}
