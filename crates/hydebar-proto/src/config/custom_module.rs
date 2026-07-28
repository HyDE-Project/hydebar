//! Definition of user provided modules driven by external commands.

use std::collections::HashMap;

use serde::Deserialize;
use serde_with::serde_as;

use super::{appearance::AppearanceColor, serde_helpers::RegexCfg};

/// A module whose content is produced by an external command.
#[serde_as]
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomModuleDef {
    pub name:           String,
    /// Command run when the module is pressed with the left mouse button.
    pub command:        String,
    /// Command run when the module is pressed with the right mouse button.
    ///
    /// Left unset the module ignores right presses.
    #[serde(default)]
    pub command_right:  Option<String>,
    /// Command run when the module is pressed with the middle mouse button.
    ///
    /// Left unset the module ignores middle presses.
    #[serde(default)]
    pub command_middle: Option<String>,
    #[serde(default)]
    pub icon:           Option<String>,

    /// Yields json lines containing text, alt and an optional tooltip.
    pub listen_cmd: Option<String>,
    /// Command producing a single json object per run, the Waybar `exec`
    /// equivalent.
    ///
    /// Unlike [`CustomModuleDef::listen_cmd`] the process is expected to
    /// terminate: the bar runs it once at startup, again on every
    /// [`CustomModuleDef::interval`] tick and whenever
    /// [`CustomModuleDef::signal`] is delivered. When unset but a schedule is
    /// configured, [`CustomModuleDef::listen_cmd`] is executed instead.
    pub exec:       Option<String>,
    /// Seconds between two runs of the producing command.
    ///
    /// Leaving it unset keeps the streaming listener behaviour, where a single
    /// long lived process feeds the module.
    pub interval:   Option<u64>,
    /// Real time signal offset refreshing the module on demand.
    ///
    /// `signal = 20` makes `pkill -RTMIN+20 hydebar` re-run the producing
    /// command immediately, matching the Waybar contract HyDE scripts rely on.
    pub signal:     Option<u8>,
    /// Map of regex to icon.
    pub icons:      Option<HashMap<RegexCfg, String>>,
    /// Map of regex to the color the module paints itself with, matched against
    /// the alternate state the listener reports.
    pub colors:     Option<HashMap<RegexCfg, AppearanceColor>>,
    /// Regex selecting output that raises an alert.
    pub alert:      Option<RegexCfg>
}

/// How the content of a custom module is produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomModuleSource<'a> {
    /// A long lived process streaming one json object per output line.
    Stream {
        /// Command spawned once and read until it exits.
        command: &'a str
    },
    /// A short lived process re-run on a schedule or on demand.
    Poll {
        /// Command spawned for every refresh.
        command:  &'a str,
        /// Seconds between two scheduled runs, absent for refresh on demand
        /// only.
        interval: Option<u64>,
        /// Real time signal offset forcing an immediate run.
        signal:   Option<u8>
    }
}

impl CustomModuleDef {
    /// Resolves which command feeds the module and how often it runs.
    ///
    /// Declaring `exec`, `interval` or `signal` selects the polling mode; a
    /// definition carrying only `listen_cmd` keeps streaming. A module without
    /// any producing command yields [`None`] and renders as a plain launcher.
    ///
    /// # Examples
    ///
    /// ```
    /// use hydebar_proto::config::{CustomModuleDef, CustomModuleSource};
    ///
    /// let definition: CustomModuleDef = toml::from_str(
    ///     r#"
    ///     name = "cpuinfo"
    ///     command = ""
    ///     exec = "hyde-shell cpuinfo"
    ///     interval = 5
    ///     "#
    /// )
    /// .expect("definition");
    ///
    /// assert_eq!(
    ///     definition.source(),
    ///     Some(CustomModuleSource::Poll {
    ///         command:  "hyde-shell cpuinfo",
    ///         interval: Some(5),
    ///         signal:   None
    ///     })
    /// );
    /// ```
    pub fn source(&self) -> Option<CustomModuleSource<'_>> {
        let polls = self.exec.is_some() || self.interval.is_some() || self.signal.is_some();

        let command = self
            .exec
            .as_deref()
            .or(self.listen_cmd.as_deref())
            .map(str::trim)
            .filter(|command| !command.is_empty())?;

        if polls {
            Some(CustomModuleSource::Poll {
                command,
                interval: self.interval.filter(|seconds| *seconds > 0),
                signal: self.signal
            })
        } else {
            Some(CustomModuleSource::Stream {
                command
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(config: &str) -> CustomModuleDef {
        toml::from_str(config).expect("custom module definition")
    }

    #[test]
    fn deserializes_the_schedule_fields() {
        let definition = parse(
            r#"
            name = "updates"
            command = "hyde-shell app system.update.sh up"
            exec = "hyde-shell system.update"
            interval = 86400
            signal = 20
            "#
        );

        assert_eq!(definition.exec.as_deref(), Some("hyde-shell system.update"));
        assert_eq!(definition.interval, Some(86_400));
        assert_eq!(definition.signal, Some(20));
    }

    #[test]
    fn defaults_the_schedule_fields_to_absent() {
        let definition = parse(
            r#"
            name = "notifications"
            command = "swaync-client -t -sw"
            listen_cmd = "swaync-client -swb"
            "#
        );

        assert_eq!(definition.exec, None);
        assert_eq!(definition.interval, None);
        assert_eq!(definition.signal, None);
    }

    #[test]
    fn keeps_streaming_when_no_schedule_is_configured() {
        let definition = parse(
            r#"
            name = "notifications"
            command = ""
            listen_cmd = "swaync-client -swb"
            "#
        );

        assert_eq!(
            definition.source(),
            Some(CustomModuleSource::Stream {
                command: "swaync-client -swb"
            })
        );
    }

    #[test]
    fn polls_the_listen_command_once_an_interval_is_configured() {
        let definition = parse(
            r#"
            name = "cpuinfo"
            command = ""
            listen_cmd = "hyde-shell cpuinfo"
            interval = 5
            "#
        );

        assert_eq!(
            definition.source(),
            Some(CustomModuleSource::Poll {
                command:  "hyde-shell cpuinfo",
                interval: Some(5),
                signal:   None
            })
        );
    }

    #[test]
    fn prefers_the_exec_command_over_the_listener_one() {
        let definition = parse(
            r#"
            name = "cpuinfo"
            command = ""
            listen_cmd = "ignored"
            exec = "hyde-shell cpuinfo"
            signal = 20
            "#
        );

        assert_eq!(
            definition.source(),
            Some(CustomModuleSource::Poll {
                command:  "hyde-shell cpuinfo",
                interval: None,
                signal:   Some(20)
            })
        );
    }

    #[test]
    fn treats_a_zero_interval_as_refresh_on_demand_only() {
        let definition = parse(
            r#"
            name = "cpuinfo"
            command = ""
            exec = "hyde-shell cpuinfo"
            interval = 0
            "#
        );

        assert_eq!(
            definition.source(),
            Some(CustomModuleSource::Poll {
                command:  "hyde-shell cpuinfo",
                interval: None,
                signal:   None
            })
        );
    }

    #[test]
    fn reports_no_source_without_a_producing_command() {
        let definition = parse(
            r#"
            name = "launcher"
            command = "wofi"
            "#
        );

        assert_eq!(definition.source(), None);
    }
}
