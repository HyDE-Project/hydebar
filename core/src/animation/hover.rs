//! Fade springs of the items a pointer can rest on.
//!
//! There is one pointer, but two items animate at once whenever it slides
//! from one to the next: the fade-out of the left item overlaps the fade-in
//! of the entered one, which is exactly why a single spring cannot serve a
//! bar.

use std::{collections::HashMap, hash::Hash, time::Duration};

use super::spring::Spring;

/// Fade springs of the items a pointer can rest on, one per item.
///
/// There is one pointer, but two items animate at once whenever it slides from
/// one to the next: the fade-out of the left item overlaps the fade-in of the
/// entered one, which is exactly why a single spring cannot serve a bar.
///
/// A spring is created the first time its item is pointed at and dropped once
/// it fades all the way out, so a bar nobody touches holds no springs at all.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use hydebar_core::animation::{HoverFades, SNAPPY};
///
/// let mut fades: HoverFades<&str> = HoverFades::default();
/// fades.point("clock", true, true, SNAPPY);
/// assert!(fades.is_animating());
///
/// while fades.advance(Duration::from_millis(8)) {}
/// assert_eq!(fades.progress(&"clock"), 1.0);
/// ```
#[derive(Debug, Clone)]
pub struct HoverFades<K>
where
    K: Eq + Hash
{
    springs: HashMap<K, Spring>
}

impl<K> Default for HoverFades<K>
where
    K: Eq + Hash
{
    fn default() -> Self {
        Self {
            springs: HashMap::new()
        }
    }
}

impl<K> HoverFades<K>
where
    K: Eq + Hash
{
    /// Follows the pointer entering `key`, or leaving it.
    ///
    /// With `animated` off the fade snaps to its end instead of travelling,
    /// which is the whole hover feedback the previous instant flip gave.
    pub fn point(&mut self, key: K, entered: bool, animated: bool, response: Duration) {
        if !animated && !entered {
            self.springs.remove(&key);
            return;
        }

        if !entered && !self.springs.contains_key(&key) {
            return;
        }

        let spring = self.springs.entry(key).or_insert_with(|| Spring::new(0.0));
        let target = if entered { 1.0 } else { 0.0 };

        spring.set_response(response);

        if animated {
            spring.set_target(target);
        } else {
            spring.snap_to(target);
        }
    }

    /// Sends every fade but `kept` toward out.
    ///
    /// One pointer rests on one module: an enter arriving while another
    /// module still reads as hovered means that module's leave was lost to
    /// a relayout under the pointer, and its highlight would stay glued on.
    pub fn leave_others(&mut self, kept: &K, animated: bool, response: Duration) {
        if animated {
            for (key, spring) in &mut self.springs {
                if key != kept {
                    spring.set_response(response);
                    spring.set_target(0.0);
                }
            }
        } else {
            self.springs.retain(|key, _| key == kept);
        }
    }

    /// How far the fade of `key` has travelled, zero out and one fully in.
    #[must_use]
    pub fn progress(&self, key: &K) -> f32 {
        self.springs
            .get(key)
            .map_or(0.0, |spring| spring.value().clamp(0.0, 1.0))
    }

    /// Returns whether any fade still needs frames.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.springs.values().any(Spring::is_animating)
    }

    /// Advances every fade by `elapsed` and reports whether any keeps moving.
    ///
    /// A fade settled all the way out is dropped, so the map only ever holds
    /// items the pointer rests on or is still leaving.
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        let mut moving = false;

        self.springs.retain(|_, spring| {
            let live = spring.advance(elapsed);
            moving |= live;

            live || spring.value() > 0.0
        });

        moving
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;
    use crate::animation::SNAPPY;

    fn drain(fades: &mut HoverFades<&str>) {
        let mut frames = 0;
        while fades.advance(Duration::from_millis(8)) {
            frames += 1;
            assert!(frames < 1000, "hover fades failed to settle");
        }
    }

    #[test]
    fn an_untouched_item_has_no_progress() {
        let fades: HoverFades<&str> = HoverFades::default();

        assert_eq!(fades.progress(&"clock"), 0.0);
        assert!(!fades.is_animating());
    }

    #[test]
    fn entering_fades_in_and_leaving_fades_back_out() {
        let mut fades: HoverFades<&str> = HoverFades::default();

        fades.point("clock", true, true, SNAPPY);
        let _ = fades.advance(Duration::from_millis(30));
        let midway = fades.progress(&"clock");
        assert!(midway > 0.0 && midway < 1.0);

        drain(&mut fades);
        assert_eq!(fades.progress(&"clock"), 1.0);

        fades.point("clock", false, true, SNAPPY);
        drain(&mut fades);
        assert_eq!(fades.progress(&"clock"), 0.0);
    }

    #[test]
    fn a_fade_settled_out_is_dropped() {
        let mut fades: HoverFades<&str> = HoverFades::default();

        fades.point("clock", true, true, SNAPPY);
        fades.point("clock", false, true, SNAPPY);
        drain(&mut fades);

        assert!(fades.springs.is_empty());
    }

    #[test]
    fn sliding_between_items_animates_both_at_once() {
        let mut fades: HoverFades<&str> = HoverFades::default();
        fades.point("clock", true, true, SNAPPY);
        drain(&mut fades);

        fades.point("clock", false, true, SNAPPY);
        fades.point("battery", true, true, SNAPPY);
        let _ = fades.advance(Duration::from_millis(30));

        assert!(fades.progress(&"clock") < 1.0);
        assert!(fades.progress(&"battery") > 0.0);
        assert!(fades.is_animating());
    }

    #[test]
    fn leaving_an_item_never_entered_stores_nothing() {
        let mut fades: HoverFades<&str> = HoverFades::default();

        fades.point("clock", false, true, SNAPPY);

        assert!(
            fades.springs.is_empty(),
            "a stray leave must not park a settled spring in the map"
        );
        assert!(!fades.is_animating());
    }

    #[test]
    fn disabled_animations_snap_the_fade() {
        let mut fades: HoverFades<&str> = HoverFades::default();

        fades.point("clock", true, false, SNAPPY);
        assert_eq!(fades.progress(&"clock"), 1.0);
        assert!(!fades.is_animating());

        fades.point("clock", false, false, SNAPPY);
        assert_eq!(fades.progress(&"clock"), 0.0);
        assert!(fades.springs.is_empty());
    }
}
