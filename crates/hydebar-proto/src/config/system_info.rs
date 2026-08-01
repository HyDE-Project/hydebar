//! Configuration for the system information module.
//!
//! The readouts the module can draw live in [`indicator`], the warning and
//! alert thresholds behind them in [`thresholds`]; this file keeps the module
//! configuration tying them together.

mod indicator;
mod thresholds;

pub use indicator::{MemoryFormat, SystemIndicator};
use serde::Deserialize;
pub use thresholds::{
    SystemInfoCpu, SystemInfoDisk, SystemInfoGpu, SystemInfoMemory, SystemInfoTemperature
};

/// System information module behaviour.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct SystemModuleConfig {
    /// Readouts to draw, in the order they are drawn.
    ///
    /// Leaving the list out, or leaving it empty, lets the panel decide: it
    /// draws every readout the machine actually reports and nothing else, so a
    /// fresh install shows the temperatures of whatever hardware it finds
    /// without anybody editing a file. Naming readouts here pins both the
    /// selection and the order instead.
    #[serde(default)]
    pub indicators:  Vec<SystemIndicator>,
    /// Readouts to leave out of the automatic selection.
    #[serde(default)]
    pub hide:        Vec<SystemIndicator>,
    #[serde(default)]
    pub cpu:         SystemInfoCpu,
    #[serde(default)]
    pub memory:      SystemInfoMemory,
    #[serde(default)]
    pub temperature: SystemInfoTemperature,
    #[serde(default)]
    pub gpu:         SystemInfoGpu,
    #[serde(default)]
    pub disk:        SystemInfoDisk
}

impl SystemModuleConfig {
    /// Reports whether a left click has another readout to switch to.
    #[must_use]
    pub const fn has_alternatives(&self) -> bool {
        !self.memory.format_alt.is_empty()
    }

    /// Reports whether the panel decides which readouts to draw.
    #[must_use]
    pub const fn selects_indicators(&self) -> bool {
        self.indicators.is_empty()
    }

    /// Reports whether a readout was explicitly turned off.
    #[must_use]
    pub fn hides(&self, indicator: &SystemIndicator) -> bool {
        self.hide.contains(indicator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_defaults_to_the_percentage_readout_without_alternatives() {
        let config = SystemModuleConfig::default();

        assert!(!config.has_alternatives());
        assert_eq!(config.memory.format, MemoryFormat::Percentage);
        assert!(config.memory.format_alt.is_empty());
    }

    #[test]
    fn an_untouched_configuration_lets_the_panel_choose_the_readouts() {
        let config: SystemModuleConfig = toml::from_str("").expect("system config");

        assert!(config.selects_indicators());
        assert_eq!(config, SystemModuleConfig::default());
    }

    #[test]
    fn an_empty_list_still_lets_the_panel_choose() {
        let config: SystemModuleConfig = toml::from_str("indicators = []").expect("system config");

        assert!(config.selects_indicators());
    }

    #[test]
    fn a_listed_readout_pins_the_selection_and_the_order() {
        let config: SystemModuleConfig =
            toml::from_str(r#"indicators = ["GpuTemperature", "Cpu"]"#).expect("system config");

        assert!(!config.selects_indicators());
        assert_eq!(
            config.indicators,
            vec![SystemIndicator::GpuTemperature, SystemIndicator::Cpu]
        );
    }

    #[test]
    fn the_earlier_name_of_the_processor_temperature_still_reads() {
        let config: SystemModuleConfig =
            toml::from_str(r#"indicators = ["Cpu", "Memory", "Temperature"]"#)
                .expect("system config");

        assert_eq!(config.indicators[2], SystemIndicator::CpuTemperature);
    }

    #[test]
    fn a_readout_can_be_turned_off_without_listing_every_other_one() {
        let config: SystemModuleConfig =
            toml::from_str(r#"hide = ["GpuUsage"]"#).expect("system config");

        assert!(config.selects_indicators());
        assert!(config.hides(&SystemIndicator::GpuUsage));
        assert!(!config.hides(&SystemIndicator::GpuTemperature));
    }
}
