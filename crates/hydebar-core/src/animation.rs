//! Frame-clock driven animation primitives.
//!
//! Animations are expressed as critically damped springs rather than fixed
//! duration tweens. A spring carries its own velocity, so retargeting mid
//! flight continues smoothly instead of snapping to a new interpolation curve.
//!
//! The animator never renders and never reads application state: callers feed
//! it a target, advance it once per frame with the elapsed time, and read the
//! current value while building the view.

use std::{collections::HashMap, hash::Hash, time::Duration};

/// Largest elapsed time integrated in a single [`Spring::advance`] call.
///
/// Longer gaps, caused by the compositor withholding frame callbacks while the
/// surface is hidden, are clamped so the spring cannot explode.
const MAX_STEP: f32 = 0.25;

/// Largest integration sub-step, matching a 240 Hz cadence.
///
/// Splitting the elapsed time keeps the explicit integrator stable when frames
/// are delivered late on a slow refresh cycle.
const MAX_SUBSTEP: f32 = 1.0 / 240.0;

/// Response of the spring behind immediate feedback: hovers, presses.
///
/// The three motion tokens below are the only speeds the bar moves at. One
/// scale instead of scattered constants keeps every animation in the same
/// family: what the pointer touches answers fastest, what the theme repaints
/// moves slowest, and nothing overshoots enough to read as a toy.
pub const SNAPPY: Duration = Duration::from_millis(150);

/// Response of the spring behind windows and unfolds.
pub const STANDARD: Duration = Duration::from_millis(220);

/// Response of the spring behind whole-surface changes: theme, layout.
pub const GENTLE: Duration = Duration::from_millis(320);

/// Response of the spring carrying a palette across the bar.
///
/// Far slower than [`GENTLE`] on purpose: the front has to be seen crossing
/// island after island, and it travels beside the desktop's own wallpaper
/// sweep, which takes a couple of seconds. At menu speeds the whole crossing
/// fits in two frames and reads as everything changing at once.
pub const SWEEP: Duration = Duration::from_millis(1100);

/// Default time the spring needs to travel most of the way to its target.
const DEFAULT_RESPONSE: f32 = 0.22;

/// Default distance and velocity below which the spring is considered settled.
const DEFAULT_PRECISION: f32 = 0.001;

/// A critically damped spring animating a single scalar value.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use hydebar_core::animation::Spring;
///
/// let mut spring = Spring::new(0.0);
/// spring.set_target(1.0);
/// assert!(spring.is_animating());
///
/// while spring.advance(Duration::from_millis(8)) {}
/// assert_eq!(spring.value(), 1.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Spring {
    value:         f32,
    velocity:      f32,
    target:        f32,
    response:      f32,
    damping_ratio: f32,
    precision:     f32
}

impl Spring {
    /// Creates a settled spring resting at `value`.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
            target: value,
            response: DEFAULT_RESPONSE,
            damping_ratio: 1.0,
            precision: DEFAULT_PRECISION
        }
    }

    /// Overrides the response time, the approximate duration of a full travel.
    ///
    /// Values below one millisecond are clamped to keep the integrator stable.
    #[must_use]
    pub const fn with_response(mut self, response: Duration) -> Self {
        self.response = response.as_secs_f32().max(0.001);
        self
    }

    /// Overrides the damping ratio.
    ///
    /// `1.0` is critically damped and never overshoots. Lower values add
    /// bounce, higher values slow the approach down.
    #[must_use]
    pub const fn with_damping_ratio(mut self, damping_ratio: f32) -> Self {
        self.damping_ratio = damping_ratio.max(0.0);
        self
    }

    /// Overrides the settling precision.
    ///
    /// Springs animating large ranges, such as pixel offsets, need a coarser
    /// threshold than the default tuned for normalized values.
    #[must_use]
    pub const fn with_precision(mut self, precision: f32) -> Self {
        self.precision = precision.max(f32::EPSILON);
        self
    }

    /// Returns the current value.
    #[must_use]
    pub const fn value(&self) -> f32 {
        self.value
    }

    /// Returns the value the spring is travelling towards.
    #[must_use]
    pub const fn target(&self) -> f32 {
        self.target
    }

    /// Returns the current velocity in units per second.
    #[must_use]
    pub const fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Returns whether the spring still needs frames to reach its target.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        (self.target - self.value).abs() > self.precision
            || self.velocity.abs() > self.precision / self.response
    }

    /// Replaces the response time while the spring is live.
    ///
    /// Applied when the user edits the animation duration and the config is
    /// hot reloaded.
    pub const fn set_response(&mut self, response: Duration) {
        self.response = response.as_secs_f32().max(0.001);
    }

    /// Replaces the damping ratio while the spring is live.
    ///
    /// Applied when the motion changes character between travels — a theme
    /// whose entrance bounces retunes the same spring the previous theme rode
    /// in on.
    pub const fn set_damping_ratio(&mut self, damping_ratio: f32) {
        self.damping_ratio = damping_ratio.max(0.0);
    }

    /// Points the spring at `target`, preserving the current velocity.
    pub const fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Moves the spring to `value` immediately and clears its velocity.
    ///
    /// Used when animations are disabled or when a surface becomes visible and
    /// should not replay its entrance.
    pub const fn snap_to(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
    }

    /// Integrates the spring by `elapsed` and reports whether it keeps moving.
    ///
    /// A `false` return means the value reached its target and the caller may
    /// stop requesting frames.
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        if !self.is_animating() {
            self.settle();
            return false;
        }

        let mut remaining = elapsed.as_secs_f32().min(MAX_STEP);
        let angular_frequency = std::f32::consts::TAU / self.response;
        let stiffness = angular_frequency * angular_frequency;
        let damping = 2.0 * self.damping_ratio * angular_frequency;

        #[expect(
            clippy::while_float,
            reason = "fixed-substep integration drains the remaining time to zero"
        )]
        while remaining > 0.0 {
            let step = remaining.min(MAX_SUBSTEP);
            let acceleration =
                (-stiffness).mul_add(self.value - self.target, -(damping * self.velocity));

            self.velocity += acceleration * step;
            self.value = self.velocity.mul_add(step, self.value);
            remaining -= step;
        }

        if self.is_animating() {
            true
        } else {
            self.settle();
            false
        }
    }

    /// Snaps the value onto the target and clears residual velocity.
    const fn settle(&mut self) {
        self.value = self.target;
        self.velocity = 0.0;
    }
}

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
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    fn run_to_rest(spring: &mut Spring) -> usize {
        let mut frames = 0;
        while spring.advance(Duration::from_millis(8)) {
            frames += 1;
            assert!(frames < 1000, "spring failed to settle");
        }
        frames
    }

    #[test]
    fn new_spring_is_settled() {
        let spring = Spring::new(0.5);

        assert!(!spring.is_animating());
        assert_eq!(spring.value(), 0.5);
        assert_eq!(spring.target(), 0.5);
    }

    #[test]
    fn reaches_target_exactly() {
        let mut spring = Spring::new(0.0);
        spring.set_target(1.0);

        let frames = run_to_rest(&mut spring);

        assert!(frames > 0);
        assert_eq!(spring.value(), 1.0);
        assert_eq!(spring.velocity(), 0.0);
    }

    #[test]
    fn critically_damped_spring_does_not_overshoot() {
        let mut spring = Spring::new(0.0);
        spring.set_target(1.0);

        while spring.advance(Duration::from_millis(8)) {
            assert!(spring.value() <= 1.0 + f32::EPSILON);
        }
    }

    #[test]
    fn retargeting_keeps_velocity() {
        let mut spring = Spring::new(0.0);
        spring.set_target(1.0);
        let _ = spring.advance(Duration::from_millis(50));

        let velocity_before = spring.velocity();
        spring.set_target(0.0);

        assert_eq!(spring.velocity(), velocity_before);
        assert!(velocity_before > 0.0);
    }

    #[test]
    fn advance_on_settled_spring_reports_idle() {
        let mut spring = Spring::new(1.0);

        assert!(!spring.advance(Duration::from_millis(16)));
    }

    #[test]
    fn snap_to_clears_animation() {
        let mut spring = Spring::new(0.0);
        spring.set_target(1.0);
        let _ = spring.advance(Duration::from_millis(16));

        spring.snap_to(0.25);

        assert!(!spring.is_animating());
        assert_eq!(spring.value(), 0.25);
        assert_eq!(spring.velocity(), 0.0);
    }

    #[test]
    fn long_gap_is_clamped_and_stable() {
        let mut spring = Spring::new(0.0);
        spring.set_target(1.0);

        let _ = spring.advance(Duration::from_secs(30));

        assert!(spring.value().is_finite());
        assert!(spring.value() <= 1.0 + f32::EPSILON);
    }

    #[test]
    fn slower_response_needs_more_frames() {
        let mut fast = Spring::new(0.0).with_response(Duration::from_millis(100));
        let mut slow = Spring::new(0.0).with_response(Duration::from_millis(400));

        fast.set_target(1.0);
        slow.set_target(1.0);

        assert!(run_to_rest(&mut slow) > run_to_rest(&mut fast));
    }

    #[test]
    fn underdamped_spring_overshoots() {
        let mut spring = Spring::new(0.0).with_damping_ratio(0.4);
        spring.set_target(1.0);

        let mut peak: f32 = 0.0;
        while spring.advance(Duration::from_millis(8)) {
            peak = peak.max(spring.value());
        }

        assert!(peak > 1.0);
    }

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
    fn a_sweep_with_full_spread_stays_finite() {
        assert!(sweep(0.5, 0.5, 1.0).is_finite());
        assert!(sweep(0.5, 0.5, -1.0).is_finite());
        assert_eq!(sweep(1.0, 1.0, 1.0), 1.0);
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
