//! Configuration for the updates module.

use serde::Deserialize;

/// Seconds between two scheduled update checks when none is configured.
pub const DEFAULT_CHECK_INTERVAL: u64 = 3600;

/// Upstream branch the `HyDE` clone is measured and updated against.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HydeBranch {
    /// The released line, the branch upstream documents installing from.
    #[default]
    Master,
    /// The development line, ahead of the releases and rougher.
    Dev
}

impl HydeBranch {
    /// Every branch the bar offers, in the order they are listed.
    pub const ALL: [Self; 2] = [Self::Master, Self::Dev];

    /// Name of the branch as git knows it.
    #[must_use]
    pub const fn git_name(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Dev => "dev"
        }
    }

    /// Name shown on the choice.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Master => "Master",
            Self::Dev => "Dev"
        }
    }
}

/// Commands used to query and apply system package updates.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct UpdatesModuleConfig {
    /// The command that reports what is waiting to be installed.
    pub check_cmd:      String,
    /// The command that installs what is waiting.
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
    pub check_interval: u64,
    /// Branch the `HyDE` clone follows.
    #[serde(default, alias = "hyde-branch")]
    pub hyde_branch:    HydeBranch
}

impl Default for UpdatesModuleConfig {
    fn default() -> Self {
        Self {
            check_cmd:      String::new(),
            update_cmd:     String::new(),
            check_interval: default_check_interval(),
            hyde_branch:    HydeBranch::default()
        }
    }
}

const fn default_check_interval() -> u64 {
    DEFAULT_CHECK_INTERVAL
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
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

    #[test]
    fn a_configuration_without_a_branch_follows_the_releases() {
        let config: UpdatesModuleConfig =
            toml::from_str("check_cmd = \"a\"\nupdate_cmd = \"b\"\n").expect("a valid section");

        assert_eq!(config.hyde_branch, HydeBranch::Master);
    }

    #[test]
    fn the_dev_branch_can_be_chosen() {
        let config: UpdatesModuleConfig =
            toml::from_str("check_cmd = \"a\"\nupdate_cmd = \"b\"\nhyde_branch = \"Dev\"\n")
                .expect("a valid section");

        assert_eq!(config.hyde_branch, HydeBranch::Dev);
        assert_eq!(config.hyde_branch.git_name(), "dev");
    }
}
