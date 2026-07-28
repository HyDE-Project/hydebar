//! Configuration for the idle inhibitor module.

use std::time::Duration;

use serde::Deserialize;

/// Hint shown while the session is kept awake.
fn default_tooltip_activated() -> String {
    "Caffeine Mode Active\nPrevents system from going to sleep".to_owned()
}

/// Hint shown while the session follows the power settings.
fn default_tooltip_deactivated() -> String {
    "Caffeine Mode Inactive\nSystem will follow normal power settings".to_owned()
}

/// Idle inhibitor module behaviour.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct IdleInhibitorModuleConfig {
    /// Whether the bar keeps the session awake from the moment it starts.
    #[serde(alias = "start-activated")]
    pub start_activated:     bool,
    /// Minutes an activated inhibitor lives before releasing itself.
    ///
    /// Left unset the inhibitor stays until it is toggled off again.
    pub timeout:             Option<f64>,
    /// Hint shown while the session is kept awake.
    #[serde(alias = "tooltip-format-activated")]
    pub tooltip_activated:   String,
    /// Hint shown while the session follows the power settings.
    #[serde(alias = "tooltip-format-deactivated")]
    pub tooltip_deactivated: String
}

impl IdleInhibitorModuleConfig {
    /// Hint matching `inhibited`, empty hints reported as absent.
    #[must_use]
    pub fn tooltip(&self, inhibited: bool) -> Option<&str> {
        let hint = if inhibited {
            self.tooltip_activated.as_str()
        } else {
            self.tooltip_deactivated.as_str()
        };

        (!hint.trim().is_empty()).then_some(hint)
    }

    /// How long an activation lasts before it releases itself.
    ///
    /// A missing, negative or non-finite timeout means the inhibitor never
    /// expires on its own.
    #[must_use]
    pub fn release_after(&self) -> Option<Duration> {
        self.timeout
            .filter(|minutes| minutes.is_finite() && *minutes > 0.)
            .map(|minutes| Duration::from_secs_f64(minutes * 60.))
    }
}

impl Default for IdleInhibitorModuleConfig {
    fn default() -> Self {
        Self {
            start_activated:     false,
            timeout:             None,
            tooltip_activated:   default_tooltip_activated(),
            tooltip_deactivated: default_tooltip_deactivated()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_hints_repeat_the_two_states() {
        let config = IdleInhibitorModuleConfig::default();

        assert!(
            config
                .tooltip(true)
                .expect("an activated hint")
                .starts_with("Caffeine Mode Active")
        );
        assert!(
            config
                .tooltip(false)
                .expect("a deactivated hint")
                .starts_with("Caffeine Mode Inactive")
        );
    }

    #[test]
    fn a_blank_hint_is_reported_as_absent() {
        let config: IdleInhibitorModuleConfig = toml::from_str(
            r#"
            tooltip_activated = "   "
            tooltip_deactivated = ""
            "#
        )
        .expect("idle inhibitor config");

        assert_eq!(config.tooltip(true), None);
        assert_eq!(config.tooltip(false), None);
    }

    #[test]
    fn the_waybar_key_spelling_is_accepted_as_well() {
        let config: IdleInhibitorModuleConfig = toml::from_str(
            r#"
            start-activated = true
            tooltip-format-activated = "awake"
            tooltip-format-deactivated = "asleep"
            "#
        )
        .expect("idle inhibitor config");

        assert!(config.start_activated);
        assert_eq!(config.tooltip(true), Some("awake"));
        assert_eq!(config.tooltip(false), Some("asleep"));
    }

    #[test]
    fn a_timeout_in_minutes_becomes_the_release_delay() {
        let config: IdleInhibitorModuleConfig =
            toml::from_str("timeout = 30.5").expect("idle inhibitor config");

        assert_eq!(config.release_after(), Some(Duration::from_secs_f64(1830.)));
    }

    #[test]
    fn a_missing_or_meaningless_timeout_never_releases() {
        assert_eq!(IdleInhibitorModuleConfig::default().release_after(), None);

        for timeout in [0., -5., f64::NAN, f64::INFINITY] {
            let config = IdleInhibitorModuleConfig {
                timeout: Some(timeout),
                ..IdleInhibitorModuleConfig::default()
            };

            assert_eq!(config.release_after(), None);
        }
    }
}
