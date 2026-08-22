//! Warning and alert thresholds for every hardware readout.

use serde::Deserialize;

use super::indicator::MemoryFormat;

/// Warning and alert thresholds for CPU load, in percent.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoCpu {
    #[serde(default = "default_cpu_warn_threshold")]
    /// Share of the processor in use, in percent, the reading warns at.
    pub warn_threshold:  u32,
    #[serde(default = "default_cpu_alert_threshold")]
    /// Share of the processor in use, in percent, the reading alerts at.
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

/// Warning and alert thresholds for memory usage, in percent.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoMemory {
    #[serde(default = "default_mem_warn_threshold")]
    /// Share of the memory in use, in percent, the reading warns at.
    pub warn_threshold:  u32,
    #[serde(default = "default_mem_alert_threshold")]
    /// Share of the memory in use, in percent, the reading alerts at.
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
    /// Temperature, in degrees Celsius, the reading warns at.
    pub warn_threshold:  i32,
    #[serde(default = "default_temp_alert_threshold")]
    /// Temperature, in degrees Celsius, the reading alerts at.
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
    /// Share of the disk in use, in percent, the reading warns at.
    pub warn_threshold:  u32,
    #[serde(default = "default_disk_alert_threshold")]
    /// Share of the disk in use, in percent, the reading alerts at.
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{super::SystemModuleConfig, *};

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
