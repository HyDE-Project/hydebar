//! Frame-clock driven animation primitives.
//!
//! Animations are expressed as critically damped springs rather than fixed
//! duration tweens. A spring carries its own velocity, so retargeting mid
//! flight continues smoothly instead of snapping to a new interpolation curve.
//!
//! The animator never renders and never reads application state: callers feed
//! it a target, advance it once per frame with the elapsed time, and read the
//! current value while building the view.

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
    pub fn new(value: f32) -> Self {
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
    pub fn with_response(mut self, response: Duration) -> Self {
        self.response = response.as_secs_f32().max(0.001);
        self
    }

    /// Overrides the damping ratio.
    ///
    /// `1.0` is critically damped and never overshoots. Lower values add
    /// bounce, higher values slow the approach down.
    #[must_use]
    pub fn with_damping_ratio(mut self, damping_ratio: f32) -> Self {
        self.damping_ratio = damping_ratio.max(0.0);
        self
    }

    /// Overrides the settling precision.
    ///
    /// Springs animating large ranges, such as pixel offsets, need a coarser
    /// threshold than the default tuned for normalized values.
    #[must_use]
    pub fn with_precision(mut self, precision: f32) -> Self {
        self.precision = precision.max(f32::EPSILON);
        self
    }

    /// Returns the current value.
    #[must_use]
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Returns the value the spring is travelling towards.
    #[must_use]
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Returns the current velocity in units per second.
    #[must_use]
    pub fn velocity(&self) -> f32 {
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
    pub fn set_response(&mut self, response: Duration) {
        self.response = response.as_secs_f32().max(0.001);
    }

    /// Points the spring at `target`, preserving the current velocity.
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Moves the spring to `value` immediately and clears its velocity.
    ///
    /// Used when animations are disabled or when a surface becomes visible and
    /// should not replay its entrance.
    pub fn snap_to(&mut self, value: f32) {
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

        while remaining > 0.0 {
            let step = remaining.min(MAX_SUBSTEP);
            let acceleration = -stiffness * (self.value - self.target) - damping * self.velocity;

            self.velocity += acceleration * step;
            self.value += self.velocity * step;
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
    fn settle(&mut self) {
        self.value = self.target;
        self.velocity = 0.0;
    }
}

#[cfg(test)]
mod tests {
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
}
