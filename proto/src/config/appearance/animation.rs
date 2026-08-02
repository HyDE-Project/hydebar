//! Animation timings the bar interpolates its transitions with.

use serde::Deserialize;

/// Animation configuration.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AnimationConfig {
    #[serde(default = "default_animations_enabled")]
    pub enabled:               bool,
    #[serde(default = "default_menu_fade_duration_ms")]
    pub menu_fade_duration_ms: u64,
    #[serde(default = "default_hover_duration_ms")]
    pub hover_duration_ms:     u64
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled:               default_animations_enabled(),
            menu_fade_duration_ms: default_menu_fade_duration_ms(),
            hover_duration_ms:     default_hover_duration_ms()
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn animation_config_default_values() {
        let config = AnimationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.menu_fade_duration_ms, 200);
        assert_eq!(config.hover_duration_ms, 100);
    }
}
