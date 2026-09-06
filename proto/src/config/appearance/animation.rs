//! Animation timings the bar interpolates its transitions with.

use serde::{Deserialize, Deserializer, de::Error as _};

/// Animation configuration.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct AnimationConfig {
    #[serde(default = "default_animations_enabled")]
    /// Whether the bar moves at all, or snaps between states.
    pub enabled:               bool,
    #[serde(default = "default_menu_fade_duration_ms")]
    /// How long a menu takes to fade in and out.
    pub menu_fade_duration_ms: u64,
    #[serde(default = "default_hover_duration_ms")]
    /// How long a module takes to answer the pointer.
    pub hover_duration_ms:     u64,
    /// Time one block of the desk is given to cross the screen and open.
    ///
    /// The whole unfolding is this once per block, so a theme sets the pace
    /// of the desk without having to know how many modules the bar carries.
    /// Left unset the built-in pace is used; a theme that wants the canvas to
    /// unfold at a stroll or to snap open says so here.
    #[serde(default = "default_desk_block_duration_ms")]
    pub desk_block_ms:         u64,
    #[serde(
        default = "default_damping_ratio",
        deserialize_with = "damping_ratio_deserializer"
    )]
    /// Damping ratio of the menu entrance spring.
    ///
    /// `1.0` rides in critically damped and never overshoots, like every
    /// other motion of the bar. Lower values let the menu overshoot its open
    /// state and settle back with a bounce; higher values slow the approach
    /// down. Hovers stay critically damped whatever this says: what the
    /// pointer touches must answer at once, never bounce.
    pub damping_ratio:         f32
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled:               default_animations_enabled(),
            menu_fade_duration_ms: default_menu_fade_duration_ms(),
            hover_duration_ms:     default_hover_duration_ms(),
            desk_block_ms:         default_desk_block_duration_ms(),
            damping_ratio:         default_damping_ratio()
        }
    }
}

const fn default_animations_enabled() -> bool {
    true
}

const fn default_menu_fade_duration_ms() -> u64 {
    200
}

const fn default_hover_duration_ms() -> u64 {
    100
}

/// Time one block of the desk is given unless a theme says otherwise.
///
/// Long enough for the eye to follow one island across a whole screen, which
/// is what the desktop conventions ask of a movement that size. The blocks
/// overlap heavily, so this is not paid once per module: a bar of a dozen of
/// them is still open before the hand has left the keyboard.
const fn default_desk_block_duration_ms() -> u64 {
    620
}

const fn default_damping_ratio() -> f32 {
    1.0
}

fn damping_ratio_deserializer<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>
{
    let value = f32::deserialize(deserializer)?;

    if value < 0.5 {
        return Err(D::Error::custom(
            "Damping ratio must be at least 0.5, or the spring overshoots enough to read as a toy"
        ));
    }

    if value > 2.0 {
        return Err(D::Error::custom("Damping ratio cannot be greater than 2.0"));
    }

    Ok(value)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;

    #[test]
    fn a_theme_sets_the_pace_of_the_desk() {
        let config: AnimationConfig =
            toml::from_str("desk_block_ms = 90").expect("animation config");

        assert_eq!(config.desk_block_ms, 90);
        assert_eq!(
            config.menu_fade_duration_ms, 200,
            "the other paces keep their defaults"
        );
    }

    #[test]
    fn animation_config_default_values() {
        let config = AnimationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.desk_block_ms, 620);
        assert_eq!(config.menu_fade_duration_ms, 200);
        assert_eq!(config.hover_duration_ms, 100);
        assert_eq!(config.damping_ratio, 1.0);
    }

    #[test]
    fn damping_ratio_deserializes_and_defaults_to_neutral() {
        let implicit: AnimationConfig =
            toml::from_str("").expect("empty animation table should deserialize");
        assert_eq!(implicit.damping_ratio, 1.0);

        let soft: AnimationConfig =
            toml::from_str("damping_ratio = 0.7").expect("damping_ratio should deserialize");
        assert_eq!(soft.damping_ratio, 0.7);
    }

    #[test]
    fn damping_ratio_rejects_out_of_bounds_values() {
        let err = toml::from_str::<AnimationConfig>("damping_ratio = 0.4")
            .expect_err("a damping ratio below 0.5 should error");
        assert!(err.to_string().contains("at least 0.5"));

        let err = toml::from_str::<AnimationConfig>("damping_ratio = 2.5")
            .expect_err("a damping ratio above 2 should error");
        assert!(err.to_string().contains("greater than 2.0"));
    }
}
