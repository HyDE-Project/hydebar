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

/// A single readout rendered by the system information module.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SystemIndicator {
    Cpu,
    Memory,
    MemorySwap,
    Temperature,
    Disk(String),
    IpAddress,
    DownloadSpeed,
    UploadSpeed
}

/// System information module behaviour.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemModuleConfig {
    #[serde(default = "default_system_indicators")]
    pub indicators:  Vec<SystemIndicator>,
    #[serde(default)]
    pub cpu:         SystemInfoCpu,
    #[serde(default)]
    pub memory:      SystemInfoMemory,
    #[serde(default)]
    pub temperature: SystemInfoTemperature,
    #[serde(default)]
    pub disk:        SystemInfoDisk
}

impl SystemModuleConfig {
    /// Reports whether a left click has another readout to switch to.
    pub fn has_alternatives(&self) -> bool {
        !self.memory.format_alt.is_empty()
    }
}

impl Default for SystemModuleConfig {
    fn default() -> Self {
        Self {
            indicators:  default_system_indicators(),
            cpu:         SystemInfoCpu::default(),
            memory:      SystemInfoMemory::default(),
            temperature: SystemInfoTemperature::default(),
            disk:        SystemInfoDisk::default()
        }
    }
}

fn default_system_indicators() -> Vec<SystemIndicator> {
    vec![
        SystemIndicator::Cpu,
        SystemIndicator::Memory,
        SystemIndicator::Temperature,
    ]
}

fn default_cpu_warn_threshold() -> u32 {
    60
}

fn default_cpu_alert_threshold() -> u32 {
    80
}

fn default_mem_warn_threshold() -> u32 {
    70
}

fn default_mem_alert_threshold() -> u32 {
    85
}

fn default_temp_warn_threshold() -> i32 {
    60
}

fn default_temp_alert_threshold() -> i32 {
    80
}

fn default_disk_warn_threshold() -> u32 {
    80
}

fn default_disk_alert_threshold() -> u32 {
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
}
