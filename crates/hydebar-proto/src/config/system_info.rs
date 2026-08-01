//! Configuration for the system information module.

use serde::Deserialize;

/// Warning and alert thresholds for CPU load, in percent.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoCpu {
    #[serde(default = "default_cpu_warn_threshold")]
    pub warn_threshold:  u32,
    #[serde(default = "default_cpu_alert_threshold")]
    pub alert_threshold: u32
}

impl Default for SystemInfoCpu {
    fn default() -> Self {
        Self {
            warn_threshold:  default_cpu_warn_threshold(),
            alert_threshold: default_cpu_alert_threshold()
        }
    }
}

/// Readout rendered by the memory indicators.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemoryFormat {
    /// Share of the total memory in use, for instance `50%`.
    #[default]
    Percentage,
    /// Amount of memory in use, for instance `7.8GB`.
    Bytes
}

/// Warning and alert thresholds for memory usage, in percent.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoMemory {
    #[serde(default = "default_mem_warn_threshold")]
    pub warn_threshold:  u32,
    #[serde(default = "default_mem_alert_threshold")]
    pub alert_threshold: u32,
    /// Readout the memory indicators show by default.
    #[serde(default)]
    pub format:          MemoryFormat,
    /// Readouts a left click cycles through.
    ///
    /// Leaving the list empty pins the indicators to [`Self::format`], so a
    /// configuration written before alternatives existed behaves as before.
    #[serde(default, alias = "format-alt")]
    pub format_alt:      Vec<MemoryFormat>
}

impl Default for SystemInfoMemory {
    fn default() -> Self {
        Self {
            warn_threshold:  default_mem_warn_threshold(),
            alert_threshold: default_mem_alert_threshold(),
            format:          MemoryFormat::default(),
            format_alt:      Vec::new()
        }
    }
}

/// Warning and alert thresholds for temperature, in degrees Celsius.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoTemperature {
    #[serde(default = "default_temp_warn_threshold")]
    pub warn_threshold:  i32,
    #[serde(default = "default_temp_alert_threshold")]
    pub alert_threshold: i32
}

impl Default for SystemInfoTemperature {
    fn default() -> Self {
        Self {
            warn_threshold:  default_temp_warn_threshold(),
            alert_threshold: default_temp_alert_threshold()
        }
    }
}

/// Warning and alert thresholds for disk usage, in percent.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoDisk {
    #[serde(default = "default_disk_warn_threshold")]
    pub warn_threshold:  u32,
    #[serde(default = "default_disk_alert_threshold")]
    pub alert_threshold: u32
}

impl Default for SystemInfoDisk {
    fn default() -> Self {
        Self {
            warn_threshold:  default_disk_warn_threshold(),
            alert_threshold: default_disk_alert_threshold()
        }
    }
}

/// Warning and alert thresholds for the graphics processor.
///
/// Graphics parts run hotter than processors under the same load, so they
/// carry thresholds of their own instead of borrowing the processor ones.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoGpu {
    /// Temperature, in degrees Celsius, the reading starts warning at.
    #[serde(default = "default_gpu_warn_threshold")]
    pub warn_threshold:        i32,
    /// Temperature, in degrees Celsius, the reading alerts at.
    #[serde(default = "default_gpu_alert_threshold")]
    pub alert_threshold:       i32,
    /// Share of the device that is busy, in percent, the load warns at.
    #[serde(default = "default_gpu_usage_warn_threshold")]
    pub usage_warn_threshold:  u32,
    /// Share of the device that is busy, in percent, the load alerts at.
    #[serde(default = "default_gpu_usage_alert_threshold")]
    pub usage_alert_threshold: u32,
    /// Device to report on when the machine has more than one.
    ///
    /// The entry names a vendor (`amd`, `intel`, `nvidia`), a driver
    /// (`amdgpu`, `nouveau`) or a placement (`discrete`, `integrated`).
    /// Leaving it out lets the panel pick, which prefers a card over the
    /// graphics built into the processor.
    #[serde(default)]
    pub device:                Option<String>
}

impl Default for SystemInfoGpu {
    fn default() -> Self {
        Self {
            warn_threshold:        default_gpu_warn_threshold(),
            alert_threshold:       default_gpu_alert_threshold(),
            usage_warn_threshold:  default_gpu_usage_warn_threshold(),
            usage_alert_threshold: default_gpu_usage_alert_threshold(),
            device:                None
        }
    }
}

/// A single readout rendered by the system information module.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SystemIndicator {
    Cpu,
    Memory,
    MemorySwap,
    /// Processor temperature.
    ///
    /// `Temperature` is the name earlier configurations used for it, and it
    /// keeps working now that the graphics has a reading of its own.
    #[serde(alias = "Temperature")]
    CpuTemperature,
    /// Graphics temperature.
    GpuTemperature,
    /// Share of the graphics processor that is busy.
    GpuUsage,
    Disk(String),
    IpAddress,
    DownloadSpeed,
    UploadSpeed
}

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

const fn default_cpu_warn_threshold() -> u32 {
    60
}

const fn default_cpu_alert_threshold() -> u32 {
    80
}

const fn default_mem_warn_threshold() -> u32 {
    70
}

const fn default_mem_alert_threshold() -> u32 {
    85
}

const fn default_temp_warn_threshold() -> i32 {
    60
}

const fn default_temp_alert_threshold() -> i32 {
    80
}

const fn default_gpu_warn_threshold() -> i32 {
    70
}

const fn default_gpu_alert_threshold() -> i32 {
    85
}

const fn default_gpu_usage_warn_threshold() -> u32 {
    70
}

const fn default_gpu_usage_alert_threshold() -> u32 {
    90
}

const fn default_disk_warn_threshold() -> u32 {
    80
}

const fn default_disk_alert_threshold() -> u32 {
    90
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
    fn memory_alternatives_are_read_from_the_configuration() {
        let config: SystemModuleConfig = toml::from_str(
            r#"
            [memory]
            format = "Bytes"
            format-alt = ["Percentage"]
            "#
        )
        .expect("system config");

        assert!(config.has_alternatives());
        assert_eq!(config.memory.format, MemoryFormat::Bytes);
        assert_eq!(config.memory.format_alt, vec![MemoryFormat::Percentage]);
        assert_eq!(config.memory.warn_threshold, default_mem_warn_threshold());
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

    #[test]
    fn the_graphics_device_can_be_pinned() {
        let config: SystemModuleConfig = toml::from_str(
            r#"
            [gpu]
            device = "nvidia"
            "#
        )
        .expect("system config");

        assert_eq!(config.gpu.device.as_deref(), Some("nvidia"));
        assert_eq!(config.gpu.warn_threshold, default_gpu_warn_threshold());
    }
}
