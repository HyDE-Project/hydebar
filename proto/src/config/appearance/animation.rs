//! Animation timings the bar interpolates its transitions with.

use serde::Deserialize;

/// Animation configuration.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
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
    pub desk_block_ms:         u64
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled:               default_animations_enabled(),
            menu_fade_duration_ms: default_menu_fade_duration_ms(),
            hover_duration_ms:     default_hover_duration_ms(),
            desk_block_ms:         default_desk_block_duration_ms()
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
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
    }
}
