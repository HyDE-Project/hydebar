//! Which readouts the bar draws, and why the rest are missing.
//!
//! The panel decides on its own: a readout appears because the machine
//! reports it, not because somebody listed it in a file. The configuration
//! is an override on top of that - it can pin the order, and it can
//! turn a readout off - and a machine that reports nothing simply draws
//! nothing.

use hydebar_proto::config::{SystemIndicator, SystemModuleConfig};

use super::data::SystemInfoData;

/// Readouts the panel offers when it chooses for itself, in drawing order.
///
/// Load and memory come from the kernel on every machine, so they always
/// appear; the temperatures and the graphics load appear where the hardware
/// reports them. The remaining readouts - a mount point, an address, a
/// transfer rate - answer a question the user has to ask first, so they are
/// drawn only once they are listed.
const AUTOMATIC: [SystemIndicator; 5] = [
    SystemIndicator::Cpu,
    SystemIndicator::Memory,
    SystemIndicator::CpuTemperature,
    SystemIndicator::GpuTemperature,
    SystemIndicator::GpuUsage
];

/// Why a readout cannot be drawn on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailable {
    /// This machine has no swap configured.
    NoSwap,
    /// No chip on this machine reports a processor temperature.
    NoCpuSensor,
    /// No graphics device on this machine reports a temperature.
    NoGpuSensor,
    /// The graphics driver on this machine publishes no load.
    NoGpuUsage,
    /// The mount point named in the configuration is not mounted.
    NoSuchDisk,
    /// No network interface reports an address or a transfer rate.
    NoNetwork
}

impl Unavailable {
    /// Sentence shown next to a readout that cannot be turned on.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NoSwap => "this machine has no swap configured",
            Self::NoCpuSensor => "this machine reports no processor temperature",
            Self::NoGpuSensor => "this machine reports no graphics temperature",
            Self::NoGpuUsage => "the graphics driver on this machine reports no load",
            Self::NoSuchDisk => "this mount point is not mounted",
            Self::NoNetwork => "no network interface reports an address"
        }
    }
}

/// A readout together with what the machine can do about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndicatorStatus {
    /// Which readout this line is.
    pub indicator:   SystemIndicator,
    /// Why the readout cannot be drawn, or [`None`] when it can.
    pub unavailable: Option<Unavailable>,
    /// The readout is being drawn right now.
    pub shown:       bool
}

impl IndicatorStatus {
    /// Reports whether the readout can be turned on at all.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.unavailable.is_none()
    }
}

/// Name of a readout as the menu spells it out.
#[must_use]
pub fn title(indicator: &SystemIndicator) -> String {
    match indicator {
        SystemIndicator::Cpu => "CPU usage".to_owned(),
        SystemIndicator::Memory => "Memory usage".to_owned(),
        SystemIndicator::MemorySwap => "Swap usage".to_owned(),
        SystemIndicator::CpuTemperature => "CPU temperature".to_owned(),
        SystemIndicator::GpuTemperature => "GPU temperature".to_owned(),
        SystemIndicator::GpuUsage => "GPU usage".to_owned(),
        SystemIndicator::Disk(mount) => format!("Disk {mount}"),
        SystemIndicator::IpAddress => "IP address".to_owned(),
        SystemIndicator::DownloadSpeed => "Download speed".to_owned(),
        SystemIndicator::UploadSpeed => "Upload speed".to_owned()
    }
}

/// Why a readout cannot be drawn, judged from the latest sample.
#[must_use]
pub fn unavailable(indicator: &SystemIndicator, data: &SystemInfoData) -> Option<Unavailable> {
    match indicator {
        SystemIndicator::Cpu | SystemIndicator::Memory => None,
        SystemIndicator::MemorySwap => {
            (data.memory_swap_total == 0).then_some(Unavailable::NoSwap)
        }
        SystemIndicator::CpuTemperature => data
            .cpu_temperature
            .is_none()
            .then_some(Unavailable::NoCpuSensor),
        SystemIndicator::GpuTemperature => data
            .gpu
            .as_ref()
            .is_none_or(|gpu| gpu.temperature.is_none())
            .then_some(Unavailable::NoGpuSensor),
        SystemIndicator::GpuUsage => data
            .gpu
            .as_ref()
            .is_none_or(|gpu| gpu.utilisation.is_none())
            .then_some(Unavailable::NoGpuUsage),
        SystemIndicator::Disk(mount) => (!data.disks.iter().any(|disk| &disk.mount == mount))
            .then_some(Unavailable::NoSuchDisk),
        SystemIndicator::IpAddress
        | SystemIndicator::DownloadSpeed
        | SystemIndicator::UploadSpeed => data.network.is_none().then_some(Unavailable::NoNetwork)
    }
}

/// Readouts to draw, in the order they are drawn.
#[must_use]
pub fn resolve(config: &SystemModuleConfig, data: &SystemInfoData) -> Vec<SystemIndicator> {
    let listed: Vec<SystemIndicator> = if config.selects_indicators() {
        AUTOMATIC.to_vec()
    } else {
        config.indicators.clone()
    };

    listed
        .into_iter()
        .filter(|indicator| !config.hides(indicator) && unavailable(indicator, data).is_none())
        .collect()
}

/// Every readout the module knows, with the state it is in on this machine.
///
/// A readout the machine cannot report is listed as unavailable together
/// with the reason, so nobody has to wonder why one they turned on
/// shows nothing.
#[must_use]
pub fn statuses(config: &SystemModuleConfig, data: &SystemInfoData) -> Vec<IndicatorStatus> {
    let shown = resolve(config, data);
    let mut known: Vec<SystemIndicator> = AUTOMATIC.to_vec();

    known.extend([
        SystemIndicator::MemorySwap,
        SystemIndicator::IpAddress,
        SystemIndicator::DownloadSpeed,
        SystemIndicator::UploadSpeed
    ]);
    known.extend(
        data.disks
            .iter()
            .map(|disk| SystemIndicator::Disk(disk.mount.clone()))
    );

    for indicator in config.indicators.iter().chain(config.hide.iter()) {
        if !known.contains(indicator) {
            known.push(indicator.clone());
        }
    }

    known
        .into_iter()
        .map(|indicator| IndicatorStatus {
            unavailable: unavailable(&indicator, data),
            shown: shown.contains(&indicator),
            indicator
        })
        .collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_proto::config::SystemInfoGpu;

    use super::*;
    use crate::modules::system_info::sensors::{GpuPlacement, GpuReadings, GpuVendor};

    fn graphics(temperature: Option<i32>, utilisation: Option<u32>) -> GpuReadings {
        GpuReadings {
            name: "amdgpu".to_owned(),
            source: Some("amdgpu edge".to_owned()),
            vendor: GpuVendor::Amd,
            placement: GpuPlacement::Integrated,
            temperature,
            utilisation,
            memory_used: None,
            memory_total: None
        }
    }

    fn machine(cpu: Option<i32>, gpu: Option<GpuReadings>) -> SystemInfoData {
        SystemInfoData {
            cpu_temperature: cpu,
            gpu,
            ..SystemInfoData::default()
        }
    }

    #[test]
    fn a_machine_with_both_sensors_draws_both_temperatures() {
        let data = machine(Some(71), Some(graphics(Some(47), Some(11))));

        assert_eq!(
            resolve(&SystemModuleConfig::default(), &data),
            vec![
                SystemIndicator::Cpu,
                SystemIndicator::Memory,
                SystemIndicator::CpuTemperature,
                SystemIndicator::GpuTemperature,
                SystemIndicator::GpuUsage
            ]
        );
    }

    #[test]
    fn a_machine_without_graphics_draws_no_graphics_readout() {
        let data = machine(Some(58), None);

        assert_eq!(
            resolve(&SystemModuleConfig::default(), &data),
            vec![
                SystemIndicator::Cpu,
                SystemIndicator::Memory,
                SystemIndicator::CpuTemperature
            ]
        );
    }

    #[test]
    fn a_machine_without_any_sensor_still_draws_load_and_memory() {
        let data = machine(None, None);

        assert_eq!(
            resolve(&SystemModuleConfig::default(), &data),
            vec![SystemIndicator::Cpu, SystemIndicator::Memory]
        );
    }

    #[test]
    fn a_hidden_readout_stays_out_of_the_automatic_selection() {
        let config = SystemModuleConfig {
            hide: vec![SystemIndicator::GpuUsage],
            ..SystemModuleConfig::default()
        };
        let data = machine(Some(71), Some(graphics(Some(47), Some(11))));

        assert!(!resolve(&config, &data).contains(&SystemIndicator::GpuUsage));
        assert!(resolve(&config, &data).contains(&SystemIndicator::GpuTemperature));
    }

    #[test]
    fn a_listed_readout_the_machine_cannot_report_is_left_out() {
        let config = SystemModuleConfig {
            indicators: vec![SystemIndicator::Cpu, SystemIndicator::GpuTemperature],
            ..SystemModuleConfig::default()
        };
        let data = machine(Some(71), None);

        assert_eq!(resolve(&config, &data), vec![SystemIndicator::Cpu]);
    }

    #[test]
    fn a_listed_order_is_kept_as_written() {
        let config = SystemModuleConfig {
            indicators: vec![SystemIndicator::GpuTemperature, SystemIndicator::Cpu],
            gpu: SystemInfoGpu::default(),
            ..SystemModuleConfig::default()
        };
        let data = machine(Some(71), Some(graphics(Some(47), None)));

        assert_eq!(
            resolve(&config, &data),
            vec![SystemIndicator::GpuTemperature, SystemIndicator::Cpu]
        );
    }

    #[test]
    fn an_unavailable_readout_is_listed_with_its_reason() {
        let data = machine(None, Some(graphics(None, Some(11))));
        let statuses = statuses(&SystemModuleConfig::default(), &data);

        let processor = statuses
            .iter()
            .find(|status| status.indicator == SystemIndicator::CpuTemperature)
            .expect("the processor temperature is a known readout");
        let graphics_temperature = statuses
            .iter()
            .find(|status| status.indicator == SystemIndicator::GpuTemperature)
            .expect("the graphics temperature is a known readout");
        let graphics_load = statuses
            .iter()
            .find(|status| status.indicator == SystemIndicator::GpuUsage)
            .expect("the graphics load is a known readout");

        assert_eq!(processor.unavailable, Some(Unavailable::NoCpuSensor));
        assert!(!processor.is_available());
        assert_eq!(
            graphics_temperature.unavailable,
            Some(Unavailable::NoGpuSensor)
        );
        assert!(graphics_load.is_available());
        assert!(graphics_load.shown);
        assert_eq!(
            Unavailable::NoGpuSensor.reason(),
            "this machine reports no graphics temperature"
        );
    }

    #[test]
    fn a_mounted_disk_is_available_and_an_unmounted_one_is_not() {
        let data = SystemInfoData {
            disks: vec![crate::modules::system_info::DiskData {
                mount:         "/".to_owned(),
                used:          60,
                total:         100,
                usage_percent: 60
            }],
            ..SystemInfoData::default()
        };

        assert_eq!(
            unavailable(&SystemIndicator::Disk("/".to_owned()), &data),
            None
        );
        assert_eq!(
            unavailable(&SystemIndicator::Disk("/data".to_owned()), &data),
            Some(Unavailable::NoSuchDisk)
        );
    }

    #[test]
    fn swap_is_unavailable_on_a_machine_without_swap() {
        let none = SystemInfoData::default();
        let some = SystemInfoData {
            memory_swap_total: 8 * 1024 * 1024 * 1024,
            ..SystemInfoData::default()
        };

        assert_eq!(
            unavailable(&SystemIndicator::MemorySwap, &none),
            Some(Unavailable::NoSwap)
        );
        assert_eq!(unavailable(&SystemIndicator::MemorySwap, &some), None);
        assert_eq!(
            Unavailable::NoSwap.reason(),
            "this machine has no swap configured"
        );
    }
}
