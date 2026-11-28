//! The critically damped spring every animation in the bar rides on.
//!
//! A spring carries its own velocity, so retargeting mid flight continues
//! smoothly instead of snapping to a new interpolation curve. The motion
//! tokens beside it are the only speeds the bar moves at: one scale instead
//! of scattered constants keeps every animation in the same family.

use std::time::Duration;

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
    precision:     f32,
    exit:          bool
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
            precision: DEFAULT_PRECISION,
            exit: false
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
    ///
    /// A fresh aim may bounce past its target when the damping allows it; a
    /// departure should use [`Spring::set_exit_target`] instead.
    pub const fn set_target(&mut self, target: f32) {
        self.target = target;
        self.exit = false;
    }

    /// Points the spring at `target` as a departure: this travel may not
    /// overshoot.
    ///
    /// A closing surface that bounces past closed flashes back into view
    /// before it disappears. The spring keeps its speed on the way out, but
    /// the moment it reaches or crosses the target it settles there, so the
    /// exit lands in one clean pass however bouncy the damping is.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use hydebar_core::animation::Spring;
    ///
    /// let mut spring = Spring::new(1.0).with_damping_ratio(0.4);
    /// spring.set_exit_target(0.0);
    ///
    /// let mut lowest = 1.0_f32;
    /// while spring.advance(Duration::from_millis(8)) {
    ///     lowest = lowest.min(spring.value());
    /// }
    ///
    /// assert_eq!(spring.value(), 0.0);
    /// assert!(lowest >= 0.0, "a departure must not dip past closed");
    /// ```
    pub const fn set_exit_target(&mut self, target: f32) {
        self.target = target;
        self.exit = true;
    }

    /// Moves the spring to `value` immediately and clears its velocity.
    ///
    /// Used when animations are disabled or when a surface becomes visible and
    /// should not replay its entrance.
    pub const fn snap_to(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
        self.exit = false;
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
        let departure_side = self.exit.then_some((self.target - self.value).signum());

        #[expect(
            clippy::while_float,
            reason = "fixed-substep integration drains the remaining time to zero"
        )]
        while remaining > 0.0 {
            let step = remaining.min(MAX_SUBSTEP);
            let acceleration =
                (-stiffness).mul_add(self.value - self.target, -(damping * self.velocity));

            self.velocity = acceleration.mul_add(step, self.velocity);
            self.value = self.velocity.mul_add(step, self.value);
            remaining -= step;

            if departure_side.is_some_and(|side| (self.target - self.value).signum() * side <= 0.0)
            {
                self.settle();
                return false;
            }
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
        self.exit = false;
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
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
    fn an_exit_spring_never_dips_past_its_target() {
        let mut spring = Spring::new(1.0).with_damping_ratio(0.4);
        spring.set_exit_target(0.0);

        let mut lowest = 1.0_f32;
        while spring.advance(Duration::from_millis(8)) {
            lowest = lowest.min(spring.value());
        }

        assert_eq!(spring.value(), 0.0);
        assert!(lowest >= 0.0, "a bouncy exit must not flash past closed");
    }

    #[test]
    fn an_exit_spring_lands_sooner_than_a_bouncy_aim() {
        let mut exit = Spring::new(1.0).with_damping_ratio(0.4);
        let mut bounce = Spring::new(1.0).with_damping_ratio(0.4);

        exit.set_exit_target(0.0);
        bounce.set_target(0.0);

        assert!(run_to_rest(&mut exit) < run_to_rest(&mut bounce));
    }

    #[test]
    fn a_fresh_aim_after_an_exit_may_bounce_again() {
        let mut spring = Spring::new(1.0).with_damping_ratio(0.4);
        spring.set_exit_target(0.0);
        run_to_rest(&mut spring);

        spring.set_target(1.0);

        let mut peak = 0.0_f32;
        while spring.advance(Duration::from_millis(8)) {
            peak = peak.max(spring.value());
        }

        assert!(
            peak > 1.0,
            "an entrance keeps the bounce the damping promises"
        );
    }

    #[test]
    fn an_exit_aimed_at_the_current_value_is_already_resting() {
        let mut spring = Spring::new(0.5);

        spring.set_exit_target(0.5);

        assert!(!spring.is_animating());
        assert!(!spring.advance(Duration::from_millis(16)));
        assert_eq!(spring.value(), 0.5);
    }
}
