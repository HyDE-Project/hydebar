use std::time::Duration;

use hex_color::HexColor;

use crate::{
    animation::Spring,
    config::{Appearance, AppearanceColor, MenuAppearance}
};

/// Cross-fade between two [`Appearance`] snapshots.
///
/// A single progress spring drives the whole palette instead of one spring per
/// colour: retargeting mid-flight snapshots the currently displayed appearance
/// as the new origin, so a hot reload landing during another hot reload still
/// blends from what the user actually sees.
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
            progress: Spring::new(1.0).with_response(crate::animation::GENTLE),
            current:  appearance
        }
    }

    /// Returns the appearance to render this frame.
    #[must_use]
    pub fn current(&self) -> &Appearance {
        &self.current
    }

    /// Returns whether the palette is still blending.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.progress.is_animating()
    }

    /// Blends towards `appearance`, or applies it immediately when `animated`
    /// is false.
    ///
    /// Colours and opacities cross-fade; discrete settings such as the layout
    /// style or the scale factor switch instantly because the compositor side
    /// of the shell has to be reconfigured for them anyway.
    pub fn set_target(&mut self, appearance: Appearance, animated: bool) {
        if !animated || appearance == self.to {
            self.from = appearance.clone();
            self.to = appearance.clone();
            self.current = appearance;
            self.progress.snap_to(1.0);
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
        let before = self.progress.value();
        let running = self.progress.advance(elapsed);

        if running || self.progress.value() != before {
            self.refresh();
        }

        running
    }

    /// Recomputes the displayed appearance for the current progress.
    fn refresh(&mut self) {
        self.current = blend_appearance(&self.from, &self.to, self.progress.value());
    }
}

/// Interpolates the animatable parts of an appearance.
fn blend_appearance(from: &Appearance, to: &Appearance, t: f32) -> Appearance {
    Appearance {
        font_name:                to.font_name.clone(),
        font_size:                to.font_size,
        radius:                   to.radius,
        height:                   to.height,
        side_padding:             to.side_padding,
        follow_hyde:              to.follow_hyde,
        auto_scale:               to.auto_scale,
        scale_factor:             to.scale_factor,
        style:                    to.style,
        opacity:                  blend_f32(from.opacity, to.opacity, t),
        bar_opacity:              blend_f32(from.bar_opacity, to.bar_opacity, t),
        menu:                     MenuAppearance {
            opacity:  blend_f32(from.menu.opacity, to.menu.opacity, t),
            backdrop: blend_f32(from.menu.backdrop, to.menu.backdrop, t)
        },
        animations:               to.animations.clone(),
        background_color:         blend_color(from.background_color, to.background_color, t),
        primary_color:            blend_color(from.primary_color, to.primary_color, t),
        secondary_color:          blend_color(from.secondary_color, to.secondary_color, t),
        success_color:            blend_color(from.success_color, to.success_color, t),
        danger_color:             blend_color(from.danger_color, to.danger_color, t),
        warning_color:            blend_color(from.warning_color, to.warning_color, t),
        text_color:               blend_color(from.text_color, to.text_color, t),
        workspace_colors:         blend_colors(&from.workspace_colors, &to.workspace_colors, t),
        special_workspace_colors: blend_optional_colors(
            from.special_workspace_colors.as_deref(),
            to.special_workspace_colors.as_deref(),
            t
        )
    }
}

fn blend_f32(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn blend_u8(from: u8, to: u8, t: f32) -> u8 {
    let blended = f32::from(from) + (f32::from(to) - f32::from(from)) * t;

    blended.round().clamp(0.0, 255.0) as u8
}

fn blend_hex(from: HexColor, to: HexColor, t: f32) -> HexColor {
    HexColor::rgba(
        blend_u8(from.r, to.r, t),
        blend_u8(from.g, to.g, t),
        blend_u8(from.b, to.b, t),
        blend_u8(from.a, to.a, t)
    )
}

/// Interpolates two palette entries.
///
/// The target decides which optional shades exist; a shade missing on one side
/// falls back to that side's base colour so the blend never jumps.
fn blend_color(from: AppearanceColor, to: AppearanceColor, t: f32) -> AppearanceColor {
    let (from_base, from_strong, from_weak, from_text) = parts(from);
    let (to_base, to_strong, to_weak, to_text) = parts(to);

    let base = blend_hex(from_base, to_base, t);

    match to {
        AppearanceColor::Simple(_) if from_strong.is_none() && from_weak.is_none() => {
            AppearanceColor::Simple(base)
        }
        _ => AppearanceColor::Complete {
            base,
            strong: blend_optional(from_strong, to_strong, from_base, to_base, t),
            weak: blend_optional(from_weak, to_weak, from_base, to_base, t),
            text: blend_optional(from_text, to_text, from_base, to_base, t)
        }
    }
}

fn parts(
    color: AppearanceColor
) -> (
    HexColor,
    Option<HexColor>,
    Option<HexColor>,
    Option<HexColor>
) {
    match color {
        AppearanceColor::Simple(base) => (base, None, None, None),
        AppearanceColor::Complete {
            base,
            strong,
            weak,
            text
        } => (base, strong, weak, text)
    }
}

fn blend_optional(
    from: Option<HexColor>,
    to: Option<HexColor>,
    from_base: HexColor,
    to_base: HexColor,
    t: f32
) -> Option<HexColor> {
    match (from, to) {
        (None, None) => None,
        (Some(from), Some(to)) => Some(blend_hex(from, to, t)),
        (Some(from), None) => Some(blend_hex(from, to_base, t)),
        (None, Some(to)) => Some(blend_hex(from_base, to, t))
    }
}

fn blend_colors(from: &[AppearanceColor], to: &[AppearanceColor], t: f32) -> Vec<AppearanceColor> {
    to.iter()
        .enumerate()
        .map(|(index, target)| match from.get(index) {
            Some(source) => blend_color(*source, *target, t),
            None => *target
        })
        .collect()
}

fn blend_optional_colors(
    from: Option<&[AppearanceColor]>,
    to: Option<&[AppearanceColor]>,
    t: f32
) -> Option<Vec<AppearanceColor>> {
    let to = to?;

    Some(blend_colors(from.unwrap_or(&[]), to, t))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn retargeting_blends_from_what_is_displayed() {
        let mut transition = AppearanceTransition::new(appearance_with_background(0));
        transition.set_target(appearance_with_background(200), true);
        let _ = transition.advance(Duration::from_millis(60));

        let displayed = transition.current().background_color;
        transition.set_target(appearance_with_background(40), true);

        assert_eq!(transition.current().background_color, displayed);

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

    #[test]
    fn optional_shades_blend_from_the_base_colour() {
        let from = AppearanceColor::Simple(hex(0));
        let to = AppearanceColor::Complete {
            base:   hex(100),
            strong: Some(hex(200)),
            weak:   None,
            text:   None
        };

        let blended = blend_color(from, to, 0.5);

        match blended {
            AppearanceColor::Complete {
                strong, ..
            } => assert_eq!(strong, Some(hex(100))),
            other => panic!("expected a complete colour, got {other:?}")
        }
    }
}
