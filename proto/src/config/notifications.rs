//! Configuration of the notification popups.

use serde::Deserialize;

/// Who draws the notification popups.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotificationSource {
    /// The bar itself, painted with the bar theme.
    ///
    /// The bar takes the notification bus for itself, so nothing else has to be
    /// installed and nothing else has to be themed to match.
    Builtin,
    /// The compositor's own notifications.
    ///
    /// Nothing is drawn by the bar and nothing else is installed: the bar takes
    /// the notification bus and hands every notification straight to the
    /// compositor, painted with the theme colours.
    Compositor,
    /// Whatever notification daemon the session already runs.
    ///
    /// The default, since a session that already has one keeps working exactly
    /// as it did until the choice is made deliberately.
    #[default]
    Daemon
}

impl NotificationSource {
    /// Every choice, in the order the settings window lists them.
    pub const ALL: [Self; 3] = [Self::Builtin, Self::Compositor, Self::Daemon];

    /// Whether the bar should claim the notification bus.
    ///
    /// Both of the sources the bar serves need the bus; only a separate daemon
    /// keeps it.
    #[must_use]
    pub const fn owns_the_bus(self) -> bool {
        matches!(self, Self::Builtin | Self::Compositor)
    }

    /// Whether the bar draws the popups itself.
    #[must_use]
    pub const fn draws_popups(self) -> bool {
        matches!(self, Self::Builtin)
    }

    /// Whether notifications are handed to the compositor.
    #[must_use]
    pub const fn hands_to_compositor(self) -> bool {
        matches!(self, Self::Compositor)
    }

    /// Name shown for this choice in the settings window.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Builtin => "Built in",
            Self::Compositor => "Hyprland",
            Self::Daemon => "System daemon"
        }
    }
}

/// Notification behaviour of the bar.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct NotificationsConfig {
    /// Who draws the popups.
    pub source: NotificationSource
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_session_keeps_its_daemon_until_told_otherwise() {
        assert_eq!(
            NotificationsConfig::default().source,
            NotificationSource::Daemon
        );
        assert!(!NotificationSource::default().owns_the_bus());
    }

    #[test]
    fn the_built_in_source_claims_the_bus() {
        assert!(NotificationSource::Builtin.owns_the_bus());
    }

    #[test]
    fn every_choice_is_named() {
        for source in NotificationSource::ALL {
            assert!(!source.label().is_empty());
        }
    }

    #[test]
    fn only_a_separate_daemon_keeps_the_bus() {
        assert!(NotificationSource::Builtin.owns_the_bus());
        assert!(NotificationSource::Compositor.owns_the_bus());
        assert!(!NotificationSource::Daemon.owns_the_bus());
    }

    #[test]
    fn each_source_draws_in_exactly_one_way() {
        assert!(NotificationSource::Builtin.draws_popups());
        assert!(!NotificationSource::Builtin.hands_to_compositor());

        assert!(!NotificationSource::Compositor.draws_popups());
        assert!(NotificationSource::Compositor.hands_to_compositor());

        assert!(!NotificationSource::Daemon.draws_popups());
        assert!(!NotificationSource::Daemon.hands_to_compositor());
    }

    #[test]
    fn the_choice_is_read_from_the_configuration() {
        let config: NotificationsConfig =
            toml::from_str("source = \"Builtin\"").expect("notifications config");

        assert_eq!(config.source, NotificationSource::Builtin);
    }
}
