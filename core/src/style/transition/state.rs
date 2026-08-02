//! The running cross-fade between two appearance snapshots.
//!
//! One progress spring drives the whole palette; the state here owns the two
//! snapshots being blended, the spring between them and the appearance being
//! displayed this frame. The arithmetic of mixing two appearances lives in
//! [`super::blend`].

use std::time::Duration;

use super::blend::blend_appearance;
use crate::{animation::Spring, config::Appearance};

/// Cross-fade between two [`Appearance`] snapshots.
///
/// A single progress spring drives the whole palette instead of one spring per
/// colour. A target landing while the blend still runs re-aims the running
/// travel instead of restarting it: a theme switch writes its files over
/// several reloads, and a transition that started over on every one of them
/// never got anywhere — the front the bar sweeps its islands with must cross
/// the bar once, towards wherever the newest reload points.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use hydebar_core::{config::Appearance, style::AppearanceTransition};
///
/// let mut transition = AppearanceTransition::new(Appearance::default());
/// assert!(!transition.is_animating());
///
/// let mut target = Appearance::default();
/// target.opacity = 0.5;
/// transition.set_target(target, true);
///
/// assert!(transition.is_animating());
/// while transition.advance(Duration::from_millis(8)) {}
/// assert_eq!(transition.current().opacity, 0.5);
/// ```
#[derive(Debug, Clone)]
pub struct AppearanceTransition {
    from:     Appearance,
    to:       Appearance,
    progress: Spring,
    current:  Appearance
}

impl AppearanceTransition {
    /// Creates a settled transition displaying `appearance`.
    #[must_use]
    pub fn new(appearance: Appearance) -> Self {
        Self {
            from:     appearance.clone(),
            to:       appearance.clone(),
            progress: Spring::new(1.0).with_response(crate::animation::SWEEP),
            current:  appearance
        }
    }

    /// Returns the appearance to render this frame.
    #[must_use]
    pub const fn current(&self) -> &Appearance {
        &self.current
    }

    /// Returns whether the palette is still blending.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.progress.is_animating()
    }

    /// Retunes the spring to the pace and character of the incoming theme.
    ///
    /// Called before aiming at a new palette, so each theme rides in on its
    /// own signature; a retune landing mid-flight only changes how the rest of
    /// the travel moves.
    pub const fn restyle(&mut self, response: Duration, damping: f32) {
        self.progress.set_response(response);
        self.progress.set_damping_ratio(damping);
    }

    /// How far the blend has travelled, zero at the old appearance and one at
    /// the new.
    #[must_use]
    pub const fn progress(&self) -> f32 {
        self.progress.value().clamp(0.0, 1.0)
    }

    /// The appearance a share `t` of the way through the running blend.
    ///
    /// What a travelling front is drawn from: every island samples the same
    /// blend at its own share, so the new palette crosses the bar as a wave
    /// instead of landing everywhere at once.
    #[must_use]
    pub fn sample(&self, t: f32) -> Appearance {
        blend_appearance(&self.from, &self.to, t.clamp(0.0, 1.0))
    }

    /// Blends towards `appearance`, or applies it immediately when `animated`
    /// is false.
    ///
    /// Colours and opacities cross-fade; discrete settings such as the layout
    /// style or the scale factor switch instantly because the compositor side
    /// of the shell has to be reconfigured for them anyway.
    ///
    /// A target the transition is already aimed at changes nothing at all,
    /// and one landing mid-flight only re-aims the running travel: a theme
    /// switch reloads the configuration several times over its run, and a
    /// blend that started over on every reload crawled to the new palette in
    /// steps too small to see.
    pub fn set_target(&mut self, appearance: Appearance, animated: bool) {
        if animated && appearance == self.to {
            return;
        }

        if !animated {
            self.from = appearance.clone();
            self.to = appearance.clone();
            self.current = appearance;
            self.progress.snap_to(1.0);
            return;
        }

        if self.progress.is_animating() {
            self.to = appearance;
            self.refresh();
            return;
        }

        self.from = self.current.clone();
        self.to = appearance;
        self.progress.snap_to(0.0);
        self.progress.set_target(1.0);
        self.refresh();
    }

    /// Advances the blend by `elapsed` and reports whether it keeps going.
    ///
    /// The displayed appearance is only rebuilt while the spring still moves:
    /// the value before the call already reflects the settled position, and a
    /// frame that arrives after settling must not pay for seven colour blends
    /// and two vectors nobody will see change.
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        self.advance_reporting(elapsed).0
    }

    /// [`AppearanceTransition::advance`], also reporting whether the
    /// displayed appearance actually moved.
    ///
    /// The second flag is what a theme cache wants: frames driven by hovers,
    /// menus or the greeting tick this transition too, and rebuilding the
    /// theme on every one of them pays a full palette derivation for colours
    /// that did not change. The settling frame reports movement, so the cache
    /// still lands exactly on the target.
    pub fn advance_reporting(&mut self, elapsed: Duration) -> (bool, bool) {
        let before = self.progress.value();
        let running = self.progress.advance(elapsed);
        #[expect(
            clippy::float_cmp,
            reason = "identity check on a value copied verbatim"
        )]
        let moved = running || self.progress.value() != before;

        if moved {
            self.refresh();
        }

        (running, moved)
    }

    /// Recomputes the displayed appearance for the current progress.
    fn refresh(&mut self) {
        self.current = blend_appearance(&self.from, &self.to, self.progress.value());
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use hex_color::HexColor;

    use super::*;
    use crate::config::AppearanceColor;

    fn hex(value: u8) -> HexColor {
        HexColor::rgb(value, value, value)
    }

    fn appearance_with_background(value: u8) -> Appearance {
        Appearance {
            background_color: AppearanceColor::Simple(hex(value)),
            ..Appearance::default()
        }
    }

    fn drain(transition: &mut AppearanceTransition) -> usize {
        let mut frames = 0;
        while transition.advance(Duration::from_millis(8)) {
            frames += 1;
            assert!(frames < 1000, "theme transition failed to settle");
        }
        frames
    }

    #[test]
    fn a_fresh_transition_is_settled() {
        let transition = AppearanceTransition::new(Appearance::default());

        assert!(!transition.is_animating());
        assert_eq!(transition.current(), &Appearance::default());
    }

    #[test]
    fn colours_cross_fade_towards_the_target() {
        let mut transition = AppearanceTransition::new(appearance_with_background(0));

        transition.set_target(appearance_with_background(200), true);
        let _ = transition.advance(Duration::from_millis(60));

        let midway = transition.current().background_color.get_base();
        assert!(midway.r > 0.0);
        assert!(midway.r < 200.0 / 255.0);

        drain(&mut transition);

        assert_eq!(
            transition.current().background_color,
            AppearanceColor::Simple(hex(200))
        );
    }

    #[test]
    fn disabled_animations_apply_the_target_at_once() {
        let mut transition = AppearanceTransition::new(appearance_with_background(0));

        transition.set_target(appearance_with_background(200), false);

        assert!(!transition.is_animating());
        assert_eq!(
            transition.current().background_color,
            AppearanceColor::Simple(hex(200))
        );
    }

    #[test]
    fn a_target_landing_mid_flight_re_aims_the_travel_without_restarting_it() {
        let mut transition = AppearanceTransition::new(appearance_with_background(0));
        transition.set_target(appearance_with_background(200), true);
        let _ = transition.advance(Duration::from_millis(60));

        let travelled = transition.progress();
        assert!(travelled > 0.0);

        transition.set_target(appearance_with_background(40), true);

        assert_eq!(
            transition.progress(),
            travelled,
            "a theme switch reloads several times; each must ride the front, not reset it"
        );

        drain(&mut transition);

        assert_eq!(
            transition.current().background_color,
            AppearanceColor::Simple(hex(40))
        );
    }

    #[test]
    fn setting_the_same_target_does_not_restart_the_blend() {
        let mut transition = AppearanceTransition::new(appearance_with_background(0));

        transition.set_target(appearance_with_background(0), true);

        assert!(!transition.is_animating());
    }

    #[test]
    fn a_repeated_target_rides_the_running_blend() {
        let mut transition = AppearanceTransition::new(appearance_with_background(0));
        transition.set_target(appearance_with_background(200), true);
        let _ = transition.advance(Duration::from_millis(60));

        let midway = transition.current().background_color;
        transition.set_target(appearance_with_background(200), true);

        assert!(
            transition.is_animating(),
            "a reload burst must not cut the blend short"
        );
        assert_eq!(transition.current().background_color, midway);

        drain(&mut transition);

        assert_eq!(
            transition.current().background_color,
            AppearanceColor::Simple(hex(200))
        );
    }

    #[test]
    fn a_sample_walks_the_blend_from_origin_to_target() {
        let mut transition = AppearanceTransition::new(appearance_with_background(0));
        transition.set_target(appearance_with_background(200), true);

        assert_eq!(
            transition.sample(0.0).background_color,
            AppearanceColor::Simple(hex(0))
        );
        assert_eq!(
            transition.sample(1.0).background_color,
            AppearanceColor::Simple(hex(200))
        );
        assert_eq!(
            transition.sample(0.5).background_color,
            AppearanceColor::Simple(hex(100))
        );
        assert_eq!(
            transition.sample(7.0).background_color,
            AppearanceColor::Simple(hex(200)),
            "the share is clamped"
        );
    }

    #[test]
    fn a_sample_ignores_how_far_the_spring_travelled() {
        let mut transition = AppearanceTransition::new(appearance_with_background(0));
        transition.set_target(appearance_with_background(200), true);
        let _ = transition.advance(Duration::from_millis(60));

        assert_eq!(
            transition.sample(0.0).background_color,
            AppearanceColor::Simple(hex(0)),
            "an island the front has not reached keeps the old palette"
        );
    }

    #[test]
    fn discrete_settings_switch_immediately() {
        let mut transition = AppearanceTransition::new(Appearance::default());
        let target = Appearance {
            scale_factor: 1.5,
            ..Appearance::default()
        };

        transition.set_target(target, true);

        assert_eq!(transition.current().scale_factor, 1.5);
    }

    #[test]
    fn the_bar_height_switches_immediately() {
        let mut transition = AppearanceTransition::new(Appearance::default());
        let target = Appearance {
            height: Some(38.0),
            background_color: AppearanceColor::Simple(hex(200)),
            ..Appearance::default()
        };

        transition.set_target(target, true);

        assert!(transition.is_animating());
        assert_eq!(transition.current().height, Some(38.0));
    }
}
