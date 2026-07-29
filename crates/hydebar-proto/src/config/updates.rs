//! Configuration for the updates module.

use serde::Deserialize;

/// Seconds between two scheduled update checks when none is configured.
pub const DEFAULT_CHECK_INTERVAL: u64 = 3600;

/// Commands used to query and apply system package updates.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct UpdatesModuleConfig {
    pub check_cmd:      String,
    pub update_cmd:     String,
    /// Seconds between two scheduled checks.
    ///
    /// Every check talks to a package database and often to a mirror, so the
    /// default is deliberately coarse: an update that lands between two checks
    /// is still there an hour later, while a bar that asks a mirror every
    /// minute is noticed by the mirror.
    #[serde(
        default = "default_check_interval",
        alias = "check-interval",
        alias = "check_interval_secs"
    )]
    pub check_interval: u64
}

impl Default for UpdatesModuleConfig {
    fn default() -> Self {
        Self {
            check_cmd:      String::new(),
            update_cmd:     String::new(),
            check_interval: default_check_interval()
        }
    }
}

fn default_check_interval() -> u64 {
    DEFAULT_CHECK_INTERVAL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_configuration_without_an_interval_checks_hourly() {
        let config: UpdatesModuleConfig =
            toml::from_str("check_cmd = \"a\"\nupdate_cmd = \"b\"\n").expect("a valid section");

        assert_eq!(config.check_interval, DEFAULT_CHECK_INTERVAL);
    }

    #[test]
    fn the_interval_can_be_spelled_either_way() {
        let dashed: UpdatesModuleConfig =
            toml::from_str("check_cmd = \"a\"\nupdate_cmd = \"b\"\ncheck-interval = 900\n")
                .expect("a valid section");
        let underscored: UpdatesModuleConfig =
            toml::from_str("check_cmd = \"a\"\nupdate_cmd = \"b\"\ncheck_interval = 900\n")
                .expect("a valid section");

        assert_eq!(dashed.check_interval, 900);
        assert_eq!(underscored.check_interval, 900);
    }
}
