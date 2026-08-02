//! The stagger that turns one travel into a front crossing a row.
//!
//! Feeding every item the same spring through [`sweep`] is what turns a flat
//! cross-fade into a wave: each item plays the same blend, offset by where it
//! stands.

/// Local share of a front travelling across a row of items.
///
/// `progress` is how far the whole travel has come, zero to one. `position`
/// places the item under the front: zero leads and one trails. `spread` is the
/// share of the travel spent on the stagger — at zero every item moves as one,
/// and the closer to one, the longer the leading item is finished before the
/// trailing one starts.
///
/// Feeding every item the same spring through this is what turns a flat
/// cross-fade into a wave: each item plays the same blend, offset by where it
/// stands.
///
/// # Examples
///
/// ```
/// use hydebar_core::animation::sweep;
///
/// assert_eq!(sweep(0.0, 0.5, 0.6), 0.0);
/// assert_eq!(sweep(1.0, 0.5, 0.6), 1.0);
/// assert!(sweep(0.5, 0.0, 0.6) > sweep(0.5, 1.0, 0.6));
/// ```
#[must_use]
pub fn sweep(progress: f32, position: f32, spread: f32) -> f32 {
    let spread = spread.clamp(0.0, 0.95);
    let start = position.clamp(0.0, 1.0) * spread;

    ((progress.clamp(0.0, 1.0) - start) / (1.0 - spread)).clamp(0.0, 1.0)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    #[test]
    fn a_zero_spread_moves_every_position_as_one() {
        for position in [0.0, 0.3, 1.0] {
            assert_eq!(sweep(0.4, position, 0.0), 0.4);
        }
    }

    #[test]
    fn the_leading_position_finishes_before_the_trailing_one_starts() {
        let spread = 0.6;

        assert_eq!(sweep(0.5, 0.0, spread), 1.0, "the leader is done midway");
        assert_eq!(
            sweep(0.5, 1.0, spread),
            0.0,
            "the trailer has not started midway"
        );
    }

    #[test]
    fn a_sweep_is_clamped_at_both_ends() {
        assert_eq!(sweep(-0.5, 0.5, 0.6), 0.0);
        assert_eq!(sweep(1.5, 0.5, 0.6), 1.0);
        assert_eq!(sweep(1.0, 1.0, 0.6), 1.0);
        assert_eq!(sweep(0.0, 0.0, 0.6), 0.0);
    }

    #[test]
    fn a_sweep_with_full_spread_stays_finite() {
        assert!(sweep(0.5, 0.5, 1.0).is_finite());
        assert!(sweep(0.5, 0.5, -1.0).is_finite());
        assert_eq!(sweep(1.0, 1.0, 1.0), 1.0);
    }
}
