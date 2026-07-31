mod data {
    use std::time::Instant;

    use itertools::Itertools;
    use sysinfo::{Disks, Networks, System};

    use super::sensors::{GpuReadings, HardwareSensors, Readings};

    /// Snapshot of network utilisation metrics captured during sampling.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct NetworkData {
        pub ip:             String,
        pub download_speed: u32,
        pub upload_speed:   u32,
        last_check:         Instant
    }

    impl NetworkData {
        /// Create a new network metric snapshot with the provided parameters.
        pub fn new(
            ip: String,
            download_speed: u32,
            upload_speed: u32,
            last_check: Instant
        ) -> Self {
            Self {
                ip,
                download_speed,
                upload_speed,
                last_check
            }
        }

        /// Instant when the underlying network totals were observed.
        pub fn last_check(&self) -> Instant {
            self.last_check
        }
    }

    /// Usage of one mounted filesystem.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DiskData {
        /// Where the filesystem is mounted.
        pub mount:         String,
        /// Bytes an allocation could no longer claim.
        pub used:          u64,
        /// Bytes the filesystem holds in total.
        pub total:         u64,
        /// Share of [`Self::total`] behind [`Self::used`], in percent.
        pub usage_percent: u32
    }

    /// Aggregated system information consumed by the UI layer.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct SystemInfoData {
        pub cpu_usage:         u32,
        /// Logical processors the load is averaged over; zero before the
        /// first real sample.
        pub cpu_count:         u32,
        pub memory_usage:      u32,
        /// Memory in use, in bytes, behind [`Self::memory_usage`].
        pub memory_used:       u64,
        /// Memory installed, in bytes.
        pub memory_total:      u64,
        pub memory_swap_usage: u32,
        /// Swap in use, in bytes, behind [`Self::memory_swap_usage`].
        pub memory_swap_used:  u64,
        /// Swap configured, in bytes; zero on a machine without swap.
        pub memory_swap_total: u64,
        /// Processor temperature, absent on a machine that reports none.
        pub cpu_temperature:   Option<i32>,
        /// Graphics readings, absent when the machine exposes no graphics
        /// device.
        pub gpu:               Option<GpuReadings>,
        pub disks:             Vec<DiskData>,
        pub network:           Option<NetworkData>
    }

    impl SystemInfoData {
        /// Reports whether two samples would paint the same readouts.
        ///
        /// [`NetworkData::last_check`] records when a sample was taken rather
        /// than anything the bar draws, and it moves on every tick, so
        /// plain equality would never hold. Excluding it lets an idle
        /// machine skip the repaint the sample would otherwise force.
        pub fn renders_same_as(&self, other: &Self) -> bool {
            let network_matches = match (self.network.as_ref(), other.network.as_ref()) {
                (Some(left), Some(right)) => {
                    left.ip == right.ip
                        && left.download_speed == right.download_speed
                        && left.upload_speed == right.upload_speed
                }
                (None, None) => true,
                _ => false
            };

            network_matches
                && self.cpu_usage == other.cpu_usage
                && self.cpu_count == other.cpu_count
                && self.memory_usage == other.memory_usage
                && self.memory_used == other.memory_used
                && self.memory_total == other.memory_total
                && self.memory_swap_usage == other.memory_swap_usage
                && self.memory_swap_used == other.memory_swap_used
                && self.memory_swap_total == other.memory_swap_total
                && self.cpu_temperature == other.cpu_temperature
                && self.gpu == other.gpu
                && self.disks == other.disks
        }
    }

    #[derive(Debug, Clone)]
    struct NetworkSnapshot {
        ip:                Option<String>,
        total_received:    u64,
        total_transmitted: u64,
        timestamp:         Instant
    }

    impl NetworkSnapshot {
        fn capture(networks: &Networks, now: Instant) -> Option<Self> {
            let (ip, total_received, total_transmitted) = networks.iter().fold(
                (None, 0_u64, 0_u64),
                |(first_ip, received, transmitted), (_, data)| {
                    let next_ip = first_ip.or_else(|| {
                        data.ip_networks()
                            .iter()
                            .sorted_by(|a, b| a.addr.cmp(&b.addr))
                            .next()
                            .map(|ip| ip.addr.to_string())
                    });

                    (
                        next_ip,
                        received + data.received(),
                        transmitted + data.transmitted()
                    )
                }
            );

            let ip = ip?;

            Some(Self {
                ip: Some(ip),
                total_received,
                total_transmitted,
                timestamp: now
            })
        }

        fn to_data(&self, previous: Option<&NetworkSnapshot>) -> NetworkData {
            let elapsed = previous
                .map(|snapshot| self.timestamp.saturating_duration_since(snapshot.timestamp))
                .unwrap_or_default();
            let seconds = elapsed.as_secs();

            let compute_speed = |current: u64, previous_total: u64| -> u32 {
                if seconds == 0 {
                    return 0;
                }

                let delta = current.saturating_sub(previous_total);
                ((delta / 1000) as u32) / (seconds as u32)
            };

            NetworkData {
                ip:             self.ip.clone().unwrap_or_else(|| "Unknown".to_string()),
                download_speed: compute_speed(
                    self.total_received,
                    previous.map_or(0, |snapshot| snapshot.total_received)
                ),
                upload_speed:   compute_speed(
                    self.total_transmitted,
                    previous.map_or(0, |snapshot| snapshot.total_transmitted)
                ),
                last_check:     self.timestamp
            }
        }
    }

    /// Samples system metrics using the [`sysinfo`] crate.
    ///
    /// Temperatures come from [`HardwareSensors`] rather than from `sysinfo`:
    /// the panel has to know which chip a reading belongs to, has to pair
    /// it with the load the graphics driver publishes elsewhere, and has to
    /// read only the two files it settled on instead of the whole subsystem
    /// on every tick.
    #[derive(Debug)]
    pub struct SystemInfoSampler {
        system:       System,
        sensors:      HardwareSensors,
        disks:        Option<Disks>,
        networks:     Option<Networks>,
        last_network: Option<NetworkSnapshot>
    }

    impl Default for SystemInfoSampler {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SystemInfoSampler {
        /// Instantiate a sampler with refreshed sysinfo collections.
        pub fn new() -> Self {
            Self {
                system:       System::new_with_specifics(
                    sysinfo::RefreshKind::nothing()
                        .with_cpu(sysinfo::CpuRefreshKind::nothing().with_cpu_usage())
                        .with_memory(sysinfo::MemoryRefreshKind::nothing().with_ram())
                ),
                sensors:      HardwareSensors::new(),
                disks:        None,
                networks:     None,
                last_network: None
            }
        }

        /// Pins the graphics device the readings come from.
        pub fn prefer_gpu(&mut self, preferred: Option<&str>) {
            self.sensors.prefer_gpu(preferred);
        }

        fn ensure_disks(&mut self) {
            if self.disks.is_none() {
                self.disks = Some(Disks::new_with_refreshed_list());
            }
        }

        fn ensure_networks(&mut self) {
            if self.networks.is_none() {
                self.networks = Some(Networks::new_with_refreshed_list());
            }
        }

        /// Capture the latest system metrics, updating internal state for
        /// subsequent samples.
        pub fn sample(&mut self) -> SystemInfoData {
            self.system
                .refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing().with_cpu_usage());
            self.system.refresh_memory();

            let cpu_usage = self.system.global_cpu_usage().floor() as u32;
            let cpu_count = self.system.cpus().len() as u32;
            let memory_total = self.system.total_memory();
            let memory_swap_total = self.system.total_swap();
            let (memory_usage, memory_used) =
                memory_share(memory_total, self.system.available_memory());
            let (memory_swap_usage, memory_swap_used) =
                memory_share(memory_swap_total, self.system.free_swap());

            SystemInfoData {
                cpu_usage,
                cpu_count,
                memory_usage,
                memory_used,
                memory_total,
                memory_swap_usage,
                memory_swap_used,
                memory_swap_total,
                ..SystemInfoData::default()
            }
        }

        pub fn sample_with_extras(&mut self) -> SystemInfoData {
            self.ensure_disks();
            self.ensure_networks();

            self.system
                .refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing().with_cpu_usage());
            self.system.refresh_memory();

            if let Some(ref mut disks) = self.disks {
                disks.refresh(true);
            }
            if let Some(ref mut networks) = self.networks {
                networks.refresh(true);
            }

            let now = Instant::now();
            let observation = self
                .networks
                .as_ref()
                .and_then(|networks| NetworkSnapshot::capture(networks, now));
            let network = observation
                .as_ref()
                .map(|snapshot| snapshot.to_data(self.last_network.as_ref()));
            self.last_network = observation;

            let cpu_usage = self.system.global_cpu_usage().floor() as u32;
            let cpu_count = self.system.cpus().len() as u32;
            let memory_total = self.system.total_memory();
            let memory_swap_total = self.system.total_swap();
            let (memory_usage, memory_used) =
                memory_share(memory_total, self.system.available_memory());
            let (memory_swap_usage, memory_swap_used) =
                memory_share(memory_swap_total, self.system.free_swap());

            let Readings {
                cpu: cpu_temperature,
                gpu
            } = self.sensors.read();

            let disks = self
                .disks
                .as_ref()
                .map(|disks| {
                    disks
                        .iter()
                        .filter(|disk| !disk.is_removable() && disk.total_space() != 0)
                        .map(|disk| {
                            let total = disk.total_space();
                            let used = total.saturating_sub(disk.available_space());

                            DiskData {
                                mount: disk.mount_point().to_string_lossy().to_string(),
                                used,
                                total,
                                usage_percent: percentage(used, total)
                            }
                        })
                        .sorted_by(|a, b| a.mount.cmp(&b.mount))
                        .collect()
                })
                .unwrap_or_default();

            SystemInfoData {
                cpu_usage,
                cpu_count,
                memory_usage,
                memory_used,
                memory_total,
                memory_swap_usage,
                memory_swap_used,
                memory_swap_total,
                cpu_temperature,
                gpu,
                disks,
                network
            }
        }
    }

    fn percentage(used: u64, total: u64) -> u32 {
        if total == 0 {
            return 0;
        }

        ((used as f32 / total as f32) * 100.) as u32
    }

    /// Share and absolute amount of a memory pool in use.
    ///
    /// `unused` is the amount a new allocation could claim, not the amount the
    /// kernel currently holds free: for RAM that is `MemAvailable`, which
    /// counts the reclaimable page cache as unused. Subtracting `MemFree`
    /// instead would bill every cached page to the user and report a
    /// machine with a warm cache as nearly full.
    fn memory_share(total: u64, unused: u64) -> (u32, u64) {
        let used = total.saturating_sub(unused);

        (percentage(used, total), used)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn snapshot_speed_zero_when_no_elapsed() {
            let timestamp = Instant::now();
            let previous = NetworkSnapshot {
                ip: Some("127.0.0.1".to_string()),
                total_received: 1024,
                total_transmitted: 2048,
                timestamp
            };
            let snapshot = NetworkSnapshot {
                ip: Some("127.0.0.1".to_string()),
                total_received: 2048,
                total_transmitted: 4096,
                timestamp
            };

            let data = snapshot.to_data(Some(&previous));

            assert_eq!(data.download_speed, 0);
            assert_eq!(data.upload_speed, 0);
        }

        #[test]
        fn percentage_handles_zero_total() {
            assert_eq!(percentage(5, 0), 0);
        }

        #[test]
        fn memory_share_reports_percentage_and_bytes() {
            assert_eq!(memory_share(1000, 250), (75, 750));
        }

        #[test]
        fn memory_share_handles_an_absent_pool() {
            assert_eq!(memory_share(0, 0), (0, 0));
        }

        #[test]
        fn memory_share_bills_only_what_cannot_be_reclaimed() {
            // 64 GiB total with 44 GiB available is 20 GiB in use, whatever the
            // page cache holds on top of the 8 GiB the kernel reports as free.
            let total = 64 * 1024 * 1024 * 1024;
            let available = 44 * 1024 * 1024 * 1024;

            assert_eq!(
                memory_share(total, available),
                (31, 20 * 1024 * 1024 * 1024)
            );
        }

        #[test]
        fn sampler_produces_data() {
            let mut sampler = SystemInfoSampler::new();
            let data = sampler.sample();

            assert!(data.cpu_usage <= 100);
            assert!(data.memory_usage <= 100);
            assert!(data.memory_swap_usage <= 100);
        }
    }
}
pub mod indicators {
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
        pub indicator:   SystemIndicator,
        /// Why the readout cannot be drawn, or [`None`] when it can.
        pub unavailable: Option<Unavailable>,
        /// The readout is being drawn right now.
        pub shown:       bool
    }

    impl IndicatorStatus {
        /// Reports whether the readout can be turned on at all.
        #[must_use]
        pub fn is_available(&self) -> bool {
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
            | SystemIndicator::UploadSpeed => {
                data.network.is_none().then_some(Unavailable::NoNetwork)
            }
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
}
mod runtime {
    use std::{sync::Arc, time::Duration};

    use log::error;
    use tokio::{
        sync::Notify,
        task::JoinHandle,
        time::{MissedTickBehavior, interval}
    };

    use super::{Message, data::SystemInfoData};
    use crate::{ModuleContext, ModuleEventSender, modules::system_info::SystemInfoSampler};

    /// Interval between system information refresh ticks.
    pub const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

    /// Cadence while the user is looking at the module or its window.
    pub const ATTENDED_INTERVAL: Duration = Duration::from_secs(1);

    /// Pause between priming the counters and the first published sample.
    ///
    /// The processor load is a difference between two readings, so the very
    /// first reading of a fresh sampler is not a load at all — publishing
    /// it painted `0%` until the second tick five seconds later. Long
    /// enough for the kernel counters to move, short enough that the bar
    /// shows honest numbers as soon as it is up.
    pub const WARMUP: Duration = Duration::from_millis(300);

    /// Source of the metrics the polling task publishes.
    ///
    /// Sampling sits behind a trait so the schedule and the deduplication can
    /// be exercised against a scripted source rather than against whatever
    /// the host machine happens to be doing.
    pub trait MetricSource: Send + 'static {
        /// Capture the current readouts.
        fn sample(&mut self) -> SystemInfoData;
    }

    impl MetricSource for SystemInfoSampler {
        fn sample(&mut self) -> SystemInfoData {
            self.sample_with_extras()
        }
    }

    /// Manages the background polling task responsible for refreshing system
    /// metrics.
    #[derive(Default)]
    pub struct PollingTask {
        handle: Option<JoinHandle<()>>,
        poke:   Option<Arc<Notify>>
    }

    impl PollingTask {
        /// Create a new polling task manager with no active background work.
        pub fn new() -> Self {
            Self {
                handle: None,
                poke:   None
            }
        }

        /// Abort any in-flight polling task.
        pub fn abort(&mut self) {
            if let Some(handle) = self.handle.take() {
                handle.abort();
            }

            self.poke = None;
        }

        /// Asks the running task for a sample right now.
        ///
        /// The window and the hover hints read the cached sample, and a
        /// cache refreshed only on the resting cadence can be seconds
        /// old the moment somebody looks at it. The sample still
        /// happens on the task's own thread, and one whose readouts
        /// match the ones on screen is still dropped, so a poke costs
        /// a repaint only when something actually changed.
        pub fn poke(&self) {
            if let Some(poke) = self.poke.as_ref() {
                poke.notify_one();
            }
        }

        /// Spawn a periodic refresh loop bound to the provided runtime context.
        ///
        /// `preferred_gpu` pins the graphics device the readings come from on a
        /// machine that has more than one; leaving it out lets the panel
        /// choose.
        pub fn spawn(
            &mut self,
            ctx: &ModuleContext,
            sender: ModuleEventSender<Message>,
            preferred_gpu: Option<&str>
        ) {
            let mut sampler = SystemInfoSampler::new();
            sampler.prefer_gpu(preferred_gpu);

            self.spawn_from(ctx, sender, sampler);
        }

        /// Spawn the refresh loop against an explicit metric source.
        ///
        /// Sampling happens here rather than in the module update so a tick
        /// whose readouts match the ones already on screen can be
        /// dropped: every event the module publishes rebuilds and
        /// repaints every surface the bar owns, and an idle machine
        /// reports the same numbers tick after tick.
        ///
        /// The first sample is taken and thrown away: it only primes the
        /// counters the loads are differences of. The first published one
        /// follows [`WARMUP`] later, so the bar shows honest numbers at
        /// once instead of a zero it would keep for a whole interval.
        pub fn spawn_from<S>(
            &mut self,
            ctx: &ModuleContext,
            sender: ModuleEventSender<Message>,
            mut source: S
        ) where
            S: MetricSource
        {
            self.abort();

            let poke = Arc::new(Notify::new());
            let poked = Arc::clone(&poke);

            let handle = ctx.runtime_handle().spawn(async move {
                let mut ticker = interval(REFRESH_INTERVAL);
                ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
                let _ = ticker.tick().await;
                let _ = source.sample();
                tokio::time::sleep(WARMUP).await;
                let mut published: Option<SystemInfoData> = None;

                loop {
                    let sample = source.sample();

                    if published
                        .as_ref()
                        .is_none_or(|previous| !previous.renders_same_as(&sample))
                    {
                        published = Some(sample.clone());

                        if let Err(err) = sender.try_send(Message::Sampled(sample)) {
                            error!("failed to publish system info refresh: {err}");
                        }
                    }

                    tokio::select! {
                        _ = ticker.tick() => {}
                        _ = poked.notified() => {}
                    }
                }
            });

            self.handle = Some(handle);
            self.poke = Some(poke);
        }
    }

    impl Drop for PollingTask {
        fn drop(&mut self) {
            self.abort();
        }
    }

    #[cfg(test)]
    mod tests {
        use std::num::NonZeroUsize;

        use tokio::{task::yield_now, time::advance};

        use super::*;
        use crate::{
            ModuleContext,
            event_bus::{BusEvent, EventBus, ModuleEvent},
            modules::system_info::Message
        };

        fn module_context() -> (ModuleContext, EventBus) {
            let capacity = NonZeroUsize::new(16).expect("non-zero capacity");
            let bus = EventBus::new(capacity);
            let ctx = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

            (ctx, bus)
        }

        /// Source reporting a CPU load that climbs by one on every sample.
        struct RisingLoad {
            next: u32
        }

        impl MetricSource for RisingLoad {
            fn sample(&mut self) -> SystemInfoData {
                let data = sample_with_cpu(self.next);
                self.next += 1;

                data
            }
        }

        /// Source reporting the same readouts forever, like an idle machine.
        struct SteadyLoad;

        impl MetricSource for SteadyLoad {
            fn sample(&mut self) -> SystemInfoData {
                sample_with_cpu(7)
            }
        }

        fn sample_with_cpu(cpu_usage: u32) -> SystemInfoData {
            SystemInfoData {
                cpu_usage,
                memory_usage: 42,
                memory_used: 1024,
                ..SystemInfoData::default()
            }
        }

        fn expect_cpu_usage(event: Option<BusEvent>, expected: u32) {
            match event {
                Some(BusEvent::Module(ModuleEvent::SystemInfo(Message::Sampled(data)))) => {
                    assert_eq!(data.cpu_usage, expected);
                }
                other => panic!("unexpected event: {other:?}")
            }
        }

        #[tokio::test(start_paused = true)]
        async fn schedules_periodic_refreshes() {
            let (ctx, bus) = module_context();
            let mut polling = PollingTask::default();
            let mut receiver = bus.receiver();

            let sender = ctx.module_sender(ModuleEvent::SystemInfo);
            polling.spawn_from(
                &ctx,
                sender,
                RisingLoad {
                    next: 0
                }
            );
            yield_now().await;

            assert!(receiver.try_recv().expect("initial queue state").is_none());

            advance(WARMUP).await;
            yield_now().await;

            let event = receiver.try_recv().expect("queued refresh after warmup");
            expect_cpu_usage(event, 1);

            advance(REFRESH_INTERVAL).await;
            yield_now().await;

            let event = receiver.try_recv().expect("queued refresh after interval");
            expect_cpu_usage(event, 2);
        }

        #[tokio::test(start_paused = true)]
        async fn the_priming_sample_is_never_published() {
            let (ctx, bus) = module_context();
            let mut polling = PollingTask::default();
            let mut receiver = bus.receiver();

            let sender = ctx.module_sender(ModuleEvent::SystemInfo);
            polling.spawn_from(
                &ctx,
                sender,
                RisingLoad {
                    next: 0
                }
            );
            yield_now().await;
            advance(WARMUP).await;
            yield_now().await;

            let event = receiver.try_recv().expect("first published refresh");
            expect_cpu_usage(event, 1);
        }

        #[tokio::test(start_paused = true)]
        async fn unchanged_readouts_publish_nothing() {
            let (ctx, bus) = module_context();
            let mut polling = PollingTask::default();
            let mut receiver = bus.receiver();

            let sender = ctx.module_sender(ModuleEvent::SystemInfo);
            polling.spawn_from(&ctx, sender, SteadyLoad);
            yield_now().await;

            advance(WARMUP).await;
            yield_now().await;

            let first = receiver.try_recv().expect("first refresh after warmup");
            expect_cpu_usage(first, 7);

            advance(REFRESH_INTERVAL).await;
            yield_now().await;
            advance(REFRESH_INTERVAL).await;
            yield_now().await;

            assert!(
                receiver
                    .try_recv()
                    .expect("queue state after steady samples")
                    .is_none()
            );
        }

        #[tokio::test(start_paused = true)]
        async fn a_poke_samples_without_waiting_for_the_tick() {
            let (ctx, bus) = module_context();
            let mut polling = PollingTask::default();
            let mut receiver = bus.receiver();

            let sender = ctx.module_sender(ModuleEvent::SystemInfo);
            polling.spawn_from(
                &ctx,
                sender,
                RisingLoad {
                    next: 0
                }
            );
            yield_now().await;
            advance(WARMUP).await;
            yield_now().await;

            let first = receiver.try_recv().expect("refresh after warmup");
            expect_cpu_usage(first, 1);

            polling.poke();
            yield_now().await;

            let poked = receiver.try_recv().expect("refresh after poke");
            expect_cpu_usage(poked, 2);
        }

        #[test]
        fn a_poke_without_a_running_task_is_a_no_op() {
            PollingTask::default().poke();
        }

        #[tokio::test(start_paused = true)]
        async fn respawn_replaces_previous_task() {
            let (ctx, bus) = module_context();
            let mut polling = PollingTask::default();
            let mut receiver = bus.receiver();

            let sender = ctx.module_sender(ModuleEvent::SystemInfo);
            polling.spawn_from(
                &ctx,
                sender.clone(),
                RisingLoad {
                    next: 0
                }
            );
            yield_now().await;

            advance(WARMUP).await;
            yield_now().await;

            let first = receiver.try_recv().expect("first refresh after warmup");
            expect_cpu_usage(first, 1);
            assert!(receiver.try_recv().expect("drain first refresh").is_none());

            polling.spawn_from(
                &ctx,
                sender,
                RisingLoad {
                    next: 100
                }
            );
            yield_now().await;

            advance(WARMUP).await;
            yield_now().await;

            let second = receiver.try_recv().expect("refresh after respawn");
            expect_cpu_usage(second, 101);
            assert!(receiver.try_recv().expect("no duplicate refresh").is_none());
        }
    }
}
pub mod sensors {
    //! What the machine can be asked about its own temperature and load.
    //!
    //! The panel discovers the sensors once, keeps the files it settled on, and
    //! a refresh reads exactly those files through one reused buffer: no
    //! walk of the subsystem and no search for a chip. The set is rebuilt
    //! on a slow cadence, and at once after a read fails, so a card that
    //! woke up appears and a card that went to sleep disappears without
    //! anybody restarting the bar.
    //!
    //! A machine that publishes nothing - a virtual one, or a board whose
    //! drivers expose no thermal registers - yields no readings at all.
    //! That is an ordinary state: no indicator, no placeholder and nothing
    //! in the log.

    pub mod catalog {
        //! Which monitoring chip stands for which piece of hardware, and which
        //! of its inputs to trust.
        //!
        //! Every decision here is a table lookup: adding a chip family or a
        //! vendor is an entry in a list, never a condition spread over
        //! the module. Nothing in this file reads the machine, so the
        //! whole selection can be exercised against a written-down list
        //! of chips and input labels.

        /// Vendor behind a graphics processor.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum GpuVendor {
            Amd,
            Intel,
            Nvidia,
            /// A graphics block integrated into a system on a chip, such as the
            /// ones the ARM boards expose through their thermal
            /// zones.
            SystemOnChip
        }

        impl GpuVendor {
            /// Name the vendor is addressed by in the configuration and in the
            /// menu.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    Self::Amd => "amd",
                    Self::Intel => "intel",
                    Self::Nvidia => "nvidia",
                    Self::SystemOnChip => "soc"
                }
            }
        }

        /// Where a graphics processor sits relative to the processor package.
        ///
        /// The ordering is the preference order: a card outranks a part whose
        /// placement cannot be established, which in turn outranks the block
        /// built into the processor, so a laptop with switchable
        /// graphics reports the card whenever the card is awake.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum GpuPlacement {
            Discrete,
            Unknown,
            Integrated
        }

        impl GpuPlacement {
            /// Short tag the bar puts in front of the reading.
            ///
            /// Only the integrated block is tagged: a user looking at a machine
            /// with a card must never mistake the block inside the
            /// processor for it, while a machine with a single card
            /// gains nothing from a prefix.
            #[must_use]
            pub const fn tag(self) -> Option<&'static str> {
                match self {
                    Self::Integrated => Some("iGPU"),
                    Self::Discrete | Self::Unknown => None
                }
            }
        }

        /// Chip families whose readings stand for the processor, best tier
        /// first.
        ///
        /// The first tier is a driver bound to the processor's own thermal
        /// registers, so it reports the die itself. The second is the
        /// kernel package thermal driver, which reports the same die
        /// through a coarser interface. The third is the board thermal
        /// zone, which only tracks the processor on machines that
        /// expose nothing better, and which a virtual machine may be the sole
        /// source of.
        const CPU_CHIP_TIERS: [&[&str]; 3] = [
            &[
                "k10temp",
                "zenpower",
                "zenpower3",
                "coretemp",
                "k8temp",
                "cpu thermal",
                "cpu0 thermal",
                "soc thermal",
                "scpi sensors"
            ],
            &["x86 pkg temp"],
            &["acpitz"]
        ];

        /// Chip families that report a graphics processor, with the vendor
        /// behind them.
        ///
        /// `nvidia` is the chip the vendor kernel module registers once it grew
        /// monitoring support; `nouveau` is the in-tree driver for the same
        /// cards. `i915` and `xe` are the two Intel drivers, and `gpu
        /// thermal` is the zone name the ARM boards give the block
        /// inside their system on a chip.
        const GPU_CHIPS: [(&str, GpuVendor); 8] = [
            ("amdgpu", GpuVendor::Amd),
            ("radeon", GpuVendor::Amd),
            ("nvidia", GpuVendor::Nvidia),
            ("nouveau", GpuVendor::Nvidia),
            ("i915", GpuVendor::Intel),
            ("xe", GpuVendor::Intel),
            ("gpu thermal", GpuVendor::SystemOnChip),
            ("mali", GpuVendor::SystemOnChip)
        ];

        /// Processor inputs standing for the whole package, best first.
        ///
        /// `tdie` is the die reading itself. `tctl` is the same die shifted by
        /// the offset the cooling loop is tuned against, so it only
        /// stands in when the driver publishes no die reading. `package
        /// id` and `physical id` are the two spellings the kernel has
        /// used for the package reading, and `package` and `cpu` cover
        /// the drivers that name it plainly.
        const CPU_PACKAGE_INPUTS: [&str; 6] = [
            "tdie",
            "tctl",
            "package id",
            "physical id",
            "package",
            "cpu"
        ];

        /// Processor inputs covering a single core or a single die.
        ///
        /// A core runs hotter than the package it sits in and says nothing
        /// about the rest of the chip, so one is read only when the
        /// driver publishes no package reading at all.
        const CPU_CORE_INPUTS: [&str; 3] = ["core", "tccd", "die"];

        /// Graphics inputs, best first.
        ///
        /// `junction` and `hotspot` are the two names for the hottest spot on
        /// the die, which is the reading the vendor tools show. `die`,
        /// `gpu` and `core` are the plain die readings of the remaining
        /// drivers. `edge` is measured at the rim of the package and
        /// lags the die, so it stands in only when no die reading
        /// exists. `mem` and `vram` measure the memory next to the die rather
        /// than the die, and come last.
        const GPU_INPUTS: [&str; 8] = [
            "junction", "hotspot", "die", "gpu", "core", "edge", "mem", "vram"
        ];

        /// Rank of a per-core reading, worse than any package reading.
        pub const CORE_INPUT_RANK: u8 = 100;

        /// Rank of a reading whose name says nothing about what it measures.
        ///
        /// A driver that labels nothing still reports something useful when it
        /// is the only input of a chip the tables recognise, which is
        /// how the vendor kernel module publishes the graphics
        /// temperature.
        pub const UNNAMED_INPUT_RANK: u8 = 200;

        /// Folds a driver-supplied name into the spelling the tables carry.
        ///
        /// Kernels differ in case, in whether they separate words with a space,
        /// a dash or an underscore, and in the numeric suffix they
        /// append to the second instance of a chip, so every spelling
        /// folds onto one entry instead of the tables listing them all.
        #[must_use]
        pub fn normalise(value: &str) -> String {
            let mut folded = String::with_capacity(value.len());
            let mut pending_space = false;

            for character in value.chars() {
                if character.is_whitespace() || character == '_' || character == '-' {
                    pending_space = !folded.is_empty();
                    continue;
                }

                if pending_space {
                    folded.push(' ');
                    pending_space = false;
                }

                folded.extend(character.to_lowercase());
            }

            folded
        }

        /// Folds a chip name and drops the instance number the kernel appends.
        ///
        /// A second chip of the same family arrives as `acpitz_1`, and a
        /// machine with two processor packages arrives as `coretemp`
        /// twice, so the number belongs to the instance rather than to
        /// the family.
        #[must_use]
        pub fn normalise_chip(chip: &str) -> String {
            let folded = normalise(chip);

            match folded.rsplit_once(' ') {
                Some((base, suffix))
                    if !base.is_empty()
                        && !suffix.is_empty()
                        && suffix.bytes().all(|b| b.is_ascii_digit()) =>
                {
                    base.to_owned()
                }
                _ => folded
            }
        }

        /// Reports whether an input label carries no information about what it
        /// measures.
        ///
        /// A driver that labels nothing leaves `tempN_label` absent, and the
        /// kernel interface names the file itself `tempN`, so both
        /// forms mean the same.
        #[must_use]
        pub fn is_unnamed_input(input: &str) -> bool {
            let folded = normalise(input);

            folded.is_empty()
                || folded
                    .strip_prefix("temp")
                    .is_some_and(|rest| rest.bytes().all(|b| b.is_ascii_digit()))
        }

        /// Rank of a chip as a stand-in for the processor, lower being better.
        #[must_use]
        pub fn cpu_chip_rank(chip: &str) -> Option<u8> {
            let folded = normalise_chip(chip);

            CPU_CHIP_TIERS
                .iter()
                .position(|tier| tier.contains(&folded.as_str()))
                .map(|tier| tier as u8)
        }

        /// Vendor behind a chip that reports a graphics processor.
        #[must_use]
        pub fn gpu_vendor(chip: &str) -> Option<GpuVendor> {
            let folded = normalise_chip(chip);

            GPU_CHIPS
                .iter()
                .find(|(name, _)| *name == folded)
                .map(|(_, vendor)| *vendor)
        }

        /// Vendor behind a PCI vendor identifier, as sysfs spells it.
        #[must_use]
        pub fn gpu_vendor_from_pci(id: &str) -> Option<GpuVendor> {
            match id.trim().trim_start_matches("0x") {
                "1002" | "1022" => Some(GpuVendor::Amd),
                "8086" => Some(GpuVendor::Intel),
                "10de" => Some(GpuVendor::Nvidia),
                _ => None
            }
        }

        fn table_rank(table: &[&str], input: &str) -> Option<u8> {
            let folded = normalise(input);

            table
                .iter()
                .position(|candidate| folded.starts_with(candidate))
                .map(|position| position as u8)
        }

        /// Rank of an input as the processor reading of its chip, lower being
        /// better.
        #[must_use]
        pub fn cpu_input_rank(input: &str) -> u8 {
            if is_unnamed_input(input) {
                return UNNAMED_INPUT_RANK;
            }

            table_rank(&CPU_PACKAGE_INPUTS, input)
                .or_else(|| table_rank(&CPU_CORE_INPUTS, input).map(|_| CORE_INPUT_RANK))
                .unwrap_or(UNNAMED_INPUT_RANK)
        }

        /// Rank of an input as the graphics reading of its chip, lower being
        /// better.
        #[must_use]
        pub fn gpu_input_rank(input: &str) -> u8 {
            if is_unnamed_input(input) {
                return UNNAMED_INPUT_RANK;
            }

            table_rank(&GPU_INPUTS, input).unwrap_or(UNNAMED_INPUT_RANK)
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn names_fold_across_spellings() {
                assert_eq!(normalise("Package id 0"), "package id 0");
                assert_eq!(normalise("cpu-thermal"), "cpu thermal");
                assert_eq!(normalise("CPU_THERMAL"), "cpu thermal");
                assert_eq!(normalise("  Tctl  "), "tctl");
            }

            #[test]
            fn a_chip_keeps_its_family_across_instances() {
                assert_eq!(normalise_chip("acpitz_0"), "acpitz");
                assert_eq!(normalise_chip("coretemp"), "coretemp");
                assert_eq!(normalise_chip("x86_pkg_temp"), "x86 pkg temp");
                assert_eq!(normalise_chip("mt7925_phy0"), "mt7925 phy0");
            }

            #[test]
            fn only_processor_chips_rank_as_the_processor() {
                assert_eq!(cpu_chip_rank("k10temp"), Some(0));
                assert_eq!(cpu_chip_rank("coretemp"), Some(0));
                assert_eq!(cpu_chip_rank("x86_pkg_temp"), Some(1));
                assert_eq!(cpu_chip_rank("acpitz_0"), Some(2));
                assert_eq!(cpu_chip_rank("amdgpu"), None);
                assert_eq!(cpu_chip_rank("nvme"), None);
                assert_eq!(cpu_chip_rank("r8169_0_c100:00"), None);
                assert_eq!(cpu_chip_rank("mt7925_phy0"), None);
            }

            #[test]
            fn only_graphics_chips_carry_a_vendor() {
                assert_eq!(gpu_vendor("amdgpu"), Some(GpuVendor::Amd));
                assert_eq!(gpu_vendor("nouveau"), Some(GpuVendor::Nvidia));
                assert_eq!(gpu_vendor("nvidia"), Some(GpuVendor::Nvidia));
                assert_eq!(gpu_vendor("i915"), Some(GpuVendor::Intel));
                assert_eq!(gpu_vendor("gpu-thermal"), Some(GpuVendor::SystemOnChip));
                assert_eq!(gpu_vendor("k10temp"), None);
                assert_eq!(gpu_vendor("nvme"), None);
            }

            #[test]
            fn the_die_reading_outranks_the_control_and_the_package() {
                assert!(cpu_input_rank("Tdie") < cpu_input_rank("Tctl"));
                assert!(cpu_input_rank("Tctl") < cpu_input_rank("Package id 0"));
                assert!(cpu_input_rank("Package id 0") < cpu_input_rank("Core 0"));
                assert!(cpu_input_rank("Core 0") < cpu_input_rank("temp1"));
                assert!(
                    cpu_input_rank("Physical id 0") < cpu_input_rank("Core 0"),
                    "both spellings of the package reading beat a core"
                );
            }

            #[test]
            fn the_graphics_die_outranks_the_edge_and_the_memory() {
                assert!(gpu_input_rank("junction") < gpu_input_rank("edge"));
                assert!(gpu_input_rank("edge") < gpu_input_rank("mem"));
                assert!(gpu_input_rank("mem") < gpu_input_rank("temp1"));
            }

            #[test]
            fn an_unlabelled_input_is_recognised_in_both_forms() {
                assert!(is_unnamed_input(""));
                assert!(is_unnamed_input("temp1"));
                assert!(!is_unnamed_input("edge"));
                assert!(!is_unnamed_input("Tctl"));
            }

            #[test]
            fn no_chip_stands_for_both_the_processor_and_the_graphics() {
                // One chip in both tables would put the same reading on the bar twice,
                // once as the processor and once as the graphics, so the two tables
                // have to stay disjoint however many families they grow.
                for tier in CPU_CHIP_TIERS {
                    for chip in tier {
                        assert_eq!(
                            gpu_vendor(chip),
                            None,
                            "{chip} stands for the processor already"
                        );
                    }
                }

                for (chip, _) in GPU_CHIPS {
                    assert_eq!(
                        cpu_chip_rank(chip),
                        None,
                        "{chip} stands for the graphics already"
                    );
                }
            }

            #[test]
            fn pci_identifiers_resolve_to_vendors() {
                assert_eq!(gpu_vendor_from_pci("0x10de"), Some(GpuVendor::Nvidia));
                assert_eq!(gpu_vendor_from_pci("0x1002"), Some(GpuVendor::Amd));
                assert_eq!(gpu_vendor_from_pci("0x8086"), Some(GpuVendor::Intel));
                assert_eq!(gpu_vendor_from_pci("0x1234"), None);
            }
        }
    }
    pub mod drm {
        //! Load and memory a graphics driver publishes next to its device.
        //!
        //! The monitoring subsystem carries temperatures only, so utilisation
        //! and video memory come from the direct rendering device the
        //! same card owns. The two are paired by the device the kernel
        //! links both to, which keeps a machine with several cards from
        //! mixing one card's load with another's temperature.

        use std::path::{Path, PathBuf};

        use super::{catalog::GpuVendor, hwmon::read_number};

        /// Location the kernel publishes rendering devices at.
        pub const DEFAULT_ROOT: &str = "/sys/class/drm";

        /// Power state of a device that is not currently in use.
        const SUSPENDED: &str = "suspended";

        /// One rendering device and the attributes it publishes.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Card {
            /// Driver bound to the card, such as `amdgpu` or `nvidia`.
            pub driver:         Option<String>,
            pub vendor:         Option<GpuVendor>,
            /// Device the card hangs off, shared with its monitoring chip.
            pub device:         Option<PathBuf>,
            /// Share of the card that is busy, in percent.
            pub busy:           Option<PathBuf>,
            pub memory_used:    Option<PathBuf>,
            pub memory_total:   Option<PathBuf>,
            /// Runtime power state, present once the driver allows the card to
            /// sleep.
            pub runtime_status: Option<PathBuf>
        }

        impl Card {
            /// Reports whether the card is asleep right now.
            ///
            /// A card that powers down between uses is the normal state of
            /// switchable graphics, and reading it would be both
            /// meaningless and enough to wake it, so the panel
            /// leaves a sleeping card alone.
            #[must_use]
            pub fn is_asleep(&self) -> bool {
                self.runtime_status
                    .as_deref()
                    .and_then(|path| std::fs::read_to_string(path).ok())
                    .is_some_and(|status| status.trim() == SUSPENDED)
            }

            /// Share of the card that is busy, in percent.
            pub fn utilisation(&self, buffer: &mut String) -> Option<u32> {
                let busy: u32 = read_number(self.busy.as_deref()?, buffer).ok()?;

                Some(busy.min(100))
            }

            /// Video memory in use and installed, in bytes.
            pub fn memory(&self, buffer: &mut String) -> Option<(u64, u64)> {
                let used: u64 = read_number(self.memory_used.as_deref()?, buffer).ok()?;
                let total: u64 = read_number(self.memory_total.as_deref()?, buffer).ok()?;

                (total > 0).then_some((used, total))
            }
        }

        /// Every rendering device the kernel publishes, in a stable order.
        #[must_use]
        pub fn scan(root: &Path) -> Vec<Card> {
            let Ok(entries) = std::fs::read_dir(root) else {
                return Vec::new();
            };

            let mut directories: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(is_card_name)
                })
                .collect();
            directories.sort();

            directories.iter().map(|path| read_card(path)).collect()
        }

        /// Reports whether a name addresses a card rather than one of its
        /// connectors.
        ///
        /// The subsystem lists connectors beside the cards as `card0-HDMI-A-1`,
        /// and a connector publishes neither load nor memory.
        fn is_card_name(name: &str) -> bool {
            name.strip_prefix("card").is_some_and(|rest| {
                !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit())
            })
        }

        fn read_card(path: &Path) -> Card {
            let device = path.join("device");
            let resolved = std::fs::canonicalize(&device).ok();

            Card {
                driver:         std::fs::canonicalize(device.join("driver")).ok().and_then(
                    |driver| {
                        driver
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(str::to_owned)
                    }
                ),
                vendor:         std::fs::read_to_string(device.join("vendor"))
                    .ok()
                    .and_then(|id| super::catalog::gpu_vendor_from_pci(&id)),
                busy:           present(device.join("gpu_busy_percent")),
                memory_used:    present(device.join("mem_info_vram_used")),
                memory_total:   present(device.join("mem_info_vram_total")),
                runtime_status: present(device.join("power/runtime_status")),
                device:         resolved
            }
        }

        fn present(path: PathBuf) -> Option<PathBuf> {
            path.exists().then_some(path)
        }

        #[cfg(test)]
        mod tests {
            use std::fs;

            use tempfile::TempDir;

            use super::*;

            fn card(root: &Path, name: &str) -> PathBuf {
                let device = root.join(name).join("device");
                fs::create_dir_all(&device).expect("create card directory");

                device
            }

            #[test]
            fn an_absent_subsystem_yields_no_cards() {
                let root = TempDir::new().expect("temporary root");

                assert!(scan(&root.path().join("missing")).is_empty());
            }

            #[test]
            fn connectors_are_not_mistaken_for_cards() {
                assert!(is_card_name("card0"));
                assert!(is_card_name("card1"));
                assert!(!is_card_name("card1-DP-1"));
                assert!(!is_card_name("card1-HDMI-A-1"));
                assert!(!is_card_name("renderD128"));
                assert!(!is_card_name("version"));
            }

            #[test]
            fn load_and_memory_are_read_when_the_driver_publishes_them() {
                let root = TempDir::new().expect("temporary root");
                let device = card(root.path(), "card1");
                fs::write(device.join("vendor"), "0x1002\n").expect("vendor");
                fs::write(device.join("gpu_busy_percent"), "11\n").expect("load");
                fs::write(device.join("mem_info_vram_used"), "9605545984\n").expect("used");
                fs::write(device.join("mem_info_vram_total"), "68719476736\n").expect("total");

                let cards = scan(root.path());
                let mut buffer = String::new();

                assert_eq!(cards.len(), 1);
                assert_eq!(cards[0].vendor, Some(GpuVendor::Amd));
                assert_eq!(cards[0].utilisation(&mut buffer), Some(11));
                assert_eq!(
                    cards[0].memory(&mut buffer),
                    Some((9_605_545_984, 68_719_476_736))
                );
                assert!(!cards[0].is_asleep());
            }

            #[test]
            fn a_card_that_publishes_nothing_reports_nothing() {
                let root = TempDir::new().expect("temporary root");
                card(root.path(), "card0");

                let cards = scan(root.path());
                let mut buffer = String::new();

                assert_eq!(cards[0].utilisation(&mut buffer), None);
                assert_eq!(cards[0].memory(&mut buffer), None);
            }

            #[test]
            fn a_sleeping_card_is_recognised() {
                let root = TempDir::new().expect("temporary root");
                let device = card(root.path(), "card0");
                fs::create_dir_all(device.join("power")).expect("power directory");
                fs::write(device.join("power/runtime_status"), "suspended\n").expect("status");

                let cards = scan(root.path());

                assert!(cards[0].is_asleep());
            }
        }
    }
    pub mod hwmon {
        //! Reading the kernel monitoring subsystem straight out of sysfs.
        //!
        //! Enumeration walks the subsystem once and keeps the path of every
        //! input it may later read, so a refresh is a read of the files
        //! that were already chosen rather than another walk. The root
        //! is a parameter, so the walk is exercised against a
        //! written-down directory tree instead of the machine the tests
        //! happen to run on.

        use std::{
            fs::{self, File},
            io::{self, Read},
            path::{Path, PathBuf}
        };

        use super::selection::ChipFacts;

        /// Location the kernel publishes the monitoring subsystem at.
        pub const DEFAULT_ROOT: &str = "/sys/class/hwmon";

        /// Voltage rail that only a graphics block inside a processor exposes.
        const NORTHBRIDGE_RAIL: &str = "vddnb";

        /// One temperature input of a chip.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct TemperatureInput {
            /// Label the driver gave the input, empty when it labelled none.
            pub label: String,
            /// File the reading is taken from.
            pub path:  PathBuf
        }

        /// One monitoring chip with everything the selection needs to judge it.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Chip {
            /// Name the driver registered, such as `k10temp` or `amdgpu`.
            pub name:   String,
            pub inputs: Vec<TemperatureInput>,
            pub facts:  ChipFacts,
            /// Device the chip hangs off, resolved through its `device` link.
            ///
            /// Two subsystems describing the same piece of hardware agree on
            /// this path, which is how a temperature is paired with
            /// the utilisation the graphics driver publishes
            /// elsewhere.
            pub device: Option<PathBuf>
        }

        /// Every chip the monitoring subsystem publishes, in a stable order.
        ///
        /// A machine without the subsystem, such as a virtual one, yields an
        /// empty list: that is an ordinary state and not a failure, so
        /// nothing is logged.
        #[must_use]
        pub fn scan(root: &Path) -> Vec<Chip> {
            let Ok(entries) = fs::read_dir(root) else {
                return Vec::new();
            };

            let mut directories: Vec<PathBuf> =
                entries.flatten().map(|entry| entry.path()).collect();
            directories.sort();

            directories
                .iter()
                .filter_map(|path| read_chip(path))
                .collect()
        }

        fn read_chip(directory: &Path) -> Option<Chip> {
            let name = read_trimmed(&directory.join("name"))?;
            let device = fs::canonicalize(directory.join("device")).ok();
            let mut inputs = Vec::new();
            let mut facts = ChipFacts::from_address(
                device
                    .as_deref()
                    .and_then(Path::file_name)
                    .and_then(|address| address.to_str())
            );

            let mut files: Vec<PathBuf> = fs::read_dir(directory)
                .ok()?
                .flatten()
                .map(|entry| entry.path())
                .collect();
            files.sort();

            for file in &files {
                let Some(file_name) = file.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };

                if let Some(index) = temperature_index(file_name) {
                    inputs.push(TemperatureInput {
                        label: read_trimmed(&directory.join(format!("temp{index}_label")))
                            .unwrap_or_default(),
                        path:  file.clone()
                    });
                } else if file_name.starts_with("fan") && file_name.ends_with("_input") {
                    facts.fan = true;
                } else if file_name.starts_with("in")
                    && file_name.ends_with("_label")
                    && read_trimmed(file)
                        .is_some_and(|label| label.eq_ignore_ascii_case(NORTHBRIDGE_RAIL))
                {
                    facts.northbridge_voltage = true;
                }
            }

            if inputs.is_empty() {
                return None;
            }

            Some(Chip {
                name,
                inputs,
                facts,
                device
            })
        }

        /// Index of the temperature input a file name addresses.
        fn temperature_index(file_name: &str) -> Option<&str> {
            let index = file_name.strip_prefix("temp")?.strip_suffix("_input")?;

            (!index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())).then_some(index)
        }

        fn read_trimmed(path: &Path) -> Option<String> {
            fs::read_to_string(path)
                .ok()
                .map(|content| content.trim().to_owned())
        }

        /// Temperature behind an input, in whole degrees Celsius.
        ///
        /// `buffer` is handed in so a refresh allocates nothing. The subsystem
        /// publishes millidegrees, yet a handful of drivers publish degrees, so
        /// a reading small enough to be nonsense as millidegrees is
        /// taken as degrees. The value is truncated rather than
        /// rounded, the same way every other reading the module shows
        /// is, so two indicators never disagree by a degree.
        pub fn read_temperature(path: &Path, buffer: &mut String) -> io::Result<i32> {
            let raw: i64 = read_number(path, buffer)?;

            Ok(to_degrees(raw))
        }

        /// Whole number behind a sysfs attribute.
        pub fn read_number<T>(path: &Path, buffer: &mut String) -> io::Result<T>
        where
            T: std::str::FromStr
        {
            buffer.clear();
            File::open(path)?.read_to_string(buffer)?;

            buffer.trim().parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "attribute is not a number")
            })
        }

        /// Folds a raw reading onto whole degrees Celsius.
        fn to_degrees(raw: i64) -> i32 {
            if raw.abs() >= 1000 {
                (raw / 1000) as i32
            } else {
                raw as i32
            }
        }

        #[cfg(test)]
        mod tests {
            use std::fs;

            use tempfile::TempDir;

            use super::*;

            fn write(path: &Path, contents: &str) {
                fs::write(path, contents).expect("write attribute");
            }

            fn chip_directory(root: &Path, index: usize, name: &str) -> PathBuf {
                let directory = root.join(format!("hwmon{index}"));
                fs::create_dir_all(&directory).expect("create chip directory");
                write(&directory.join("name"), name);

                directory
            }

            #[test]
            fn an_absent_subsystem_yields_no_chips() {
                let root = TempDir::new().expect("temporary root");

                assert!(scan(&root.path().join("missing")).is_empty());
            }

            #[test]
            fn labels_inputs_and_extra_rails_are_read() {
                let root = TempDir::new().expect("temporary root");

                let processor = chip_directory(root.path(), 0, "k10temp");
                write(&processor.join("temp1_input"), "71625\n");
                write(&processor.join("temp1_label"), "Tctl\n");

                let graphics = chip_directory(root.path(), 1, "amdgpu");
                write(&graphics.join("temp1_input"), "47000\n");
                write(&graphics.join("temp1_label"), "edge\n");
                write(&graphics.join("in1_label"), "vddnb\n");

                let zone = chip_directory(root.path(), 2, "acpitz_0");
                write(&zone.join("temp1_input"), "59000\n");

                let chips = scan(root.path());

                assert_eq!(chips.len(), 3);
                assert_eq!(chips[0].name, "k10temp");
                assert_eq!(chips[0].inputs[0].label, "Tctl");
                assert!(!chips[0].facts.northbridge_voltage);
                assert!(chips[1].facts.northbridge_voltage);
                assert_eq!(chips[2].inputs[0].label, "");
            }

            #[test]
            fn a_chip_without_a_temperature_is_left_out() {
                let root = TempDir::new().expect("temporary root");

                let power = chip_directory(root.path(), 0, "BAT0");
                write(&power.join("in0_input"), "12000");

                assert!(scan(root.path()).is_empty());
            }

            #[test]
            fn a_fan_marks_a_chip_as_owning_its_own_cooling() {
                let root = TempDir::new().expect("temporary root");

                let graphics = chip_directory(root.path(), 0, "amdgpu");
                write(&graphics.join("temp1_input"), "60000");
                write(&graphics.join("temp1_label"), "edge");
                write(&graphics.join("fan1_input"), "1200");

                let chips = scan(root.path());

                assert!(chips[0].facts.fan);
            }

            #[test]
            fn readings_arrive_in_whole_degrees_in_both_published_forms() {
                assert_eq!(to_degrees(71625), 71);
                assert_eq!(to_degrees(47000), 47);
                assert_eq!(to_degrees(56), 56);
                assert_eq!(to_degrees(-2000), -2);
                assert_eq!(to_degrees(0), 0);
            }

            #[test]
            fn a_reading_is_taken_from_the_file_it_was_discovered_at() {
                let root = TempDir::new().expect("temporary root");

                let processor = chip_directory(root.path(), 0, "coretemp");
                write(&processor.join("temp1_input"), "58000\n");
                write(&processor.join("temp1_label"), "Package id 0\n");

                let chips = scan(root.path());
                let mut buffer = String::new();

                assert_eq!(
                    read_temperature(&chips[0].inputs[0].path, &mut buffer).expect("reading"),
                    58
                );
            }

            #[test]
            fn a_missing_file_is_an_error_the_caller_can_act_on() {
                let root = TempDir::new().expect("temporary root");
                let mut buffer = String::new();

                assert!(read_temperature(&root.path().join("gone"), &mut buffer).is_err());
            }
        }
    }
    pub mod selection {
        //! Picking one reading per role out of everything the machine
        //! publishes.
        //!
        //! The selection never touches the machine: it works on chips and input
        //! labels, so the whole preference order is exercised against
        //! written-down sensor sets rather than against whatever
        //! hardware the test runs on.

        use super::catalog::{
            self, GpuPlacement, GpuVendor, cpu_chip_rank, cpu_input_rank, gpu_input_rank,
            gpu_vendor
        };

        /// Inputs of a chip that are not temperatures but tell one part from
        /// another.
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
        pub struct ChipFacts {
            /// The chip publishes the northbridge voltage rail.
            ///
            /// The graphics driver exposes that rail only for a graphics block
            /// built into the processor, which is the one signal in
            /// the monitoring subsystem that separates it from a
            /// card.
            pub northbridge_voltage: bool,
            /// The chip publishes a fan reading.
            ///
            /// A graphics block built into the processor is cooled by the
            /// processor fan and owns none, so a fan of its own
            /// points at a card.
            pub fan:                 bool,
            /// The device sits directly on the root complex of its bus.
            ///
            /// Intel builds its graphics into the root complex and gives it the
            /// same address on every generation, while a card of
            /// theirs is reached through a bridge, so the address
            /// separates the two. [`None`] marks a device
            /// whose address says nothing, such as one on a system on a chip.
            pub on_root_complex:     Option<bool>
        }

        impl ChipFacts {
            /// Facts an address on the bus tells about a device.
            ///
            /// The address is the last element of the device path, spelled
            /// `<domain>:<bus>:<device>.<function>` the way the kernel writes
            /// it.
            #[must_use]
            pub fn from_address(address: Option<&str>) -> Self {
                Self {
                    on_root_complex: address.and_then(root_complex_bus),
                    ..Self::default()
                }
            }
        }

        /// Reports whether an address names a device on the root complex of its
        /// bus.
        fn root_complex_bus(address: &str) -> Option<bool> {
            let mut parts = address.split(':');
            let _domain = parts.next()?;
            let bus = parts.next()?;

            parts.next()?;

            Some(bus.bytes().all(|byte| byte == b'0'))
        }

        /// A monitoring chip as the selection sees it.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct ChipView<'a> {
            /// Name the driver registered, such as `k10temp` or `amdgpu`.
            pub chip:   &'a str,
            /// Label of every temperature input, empty where the driver labels
            /// none.
            pub inputs: &'a [&'a str],
            pub facts:  ChipFacts
        }

        /// One chosen reading, addressed the way it was handed in.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct Pick {
            pub chip:  usize,
            pub input: usize
        }

        /// A chosen graphics reading together with what is known about the
        /// device.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct GpuPick {
            pub chip:      usize,
            pub input:     usize,
            pub vendor:    GpuVendor,
            pub placement: GpuPlacement
        }

        /// Where a graphics chip sits, judged from what it publishes.
        ///
        /// Cards driven by the vendor module and by its in-tree counterpart are
        /// always add-in parts on the machines those drivers bind to.
        /// For the remaining vendors the placement follows the extra
        /// inputs of the chip, and stays unknown when nothing separates
        /// a tile on a card from a block inside the processor, so an
        /// unknown placement is never presented as either one.
        #[must_use]
        pub fn placement(vendor: GpuVendor, facts: ChipFacts) -> GpuPlacement {
            match vendor {
                GpuVendor::Nvidia => GpuPlacement::Discrete,
                GpuVendor::SystemOnChip => GpuPlacement::Integrated,
                GpuVendor::Amd => {
                    if facts.northbridge_voltage {
                        GpuPlacement::Integrated
                    } else if facts.fan {
                        GpuPlacement::Discrete
                    } else {
                        GpuPlacement::Unknown
                    }
                }
                GpuVendor::Intel => match facts.on_root_complex {
                    Some(true) => GpuPlacement::Integrated,
                    Some(false) => GpuPlacement::Discrete,
                    None => {
                        if facts.fan {
                            GpuPlacement::Discrete
                        } else {
                            GpuPlacement::Unknown
                        }
                    }
                }
            }
        }

        /// Best processor reading among the chips handed in.
        ///
        /// The chip decides first, so a driver bound to the processor always
        /// beats the board thermal zone that reports whichever node the
        /// firmware happens to expose; the input decides second, so the
        /// package reading beats a core.
        #[must_use]
        pub fn select_cpu(chips: &[ChipView<'_>]) -> Option<Pick> {
            chips
                .iter()
                .enumerate()
                .filter_map(|(chip_index, chip)| {
                    let chip_rank = cpu_chip_rank(chip.chip)?;

                    chip.inputs
                        .iter()
                        .enumerate()
                        .map(|(input_index, input)| {
                            (
                                (chip_rank, cpu_input_rank(input)),
                                Pick {
                                    chip:  chip_index,
                                    input: input_index
                                }
                            )
                        })
                        .min_by_key(|(rank, _)| *rank)
                })
                .min_by_key(|(rank, _)| *rank)
                .map(|(_, pick)| pick)
        }

        /// Best graphics reading among the chips handed in.
        ///
        /// `preferred` names the device a user pinned in the configuration,
        /// matched against the vendor, the chip or the placement; an
        /// entry that matches nothing is ignored rather than leaving
        /// the machine without a reading.
        #[must_use]
        pub fn select_gpu(chips: &[ChipView<'_>], preferred: Option<&str>) -> Option<GpuPick> {
            let preferred = preferred.map(catalog::normalise);

            chips
                .iter()
                .enumerate()
                .filter_map(|(chip_index, chip)| {
                    let vendor = gpu_vendor(chip.chip)?;
                    let placement = placement(vendor, chip.facts);
                    let pinned = preferred
                        .as_deref()
                        .is_some_and(|wanted| matches(wanted, chip.chip, vendor, placement));

                    chip.inputs
                        .iter()
                        .enumerate()
                        .map(|(input_index, input)| {
                            (
                                (u8::from(!pinned), placement, gpu_input_rank(input)),
                                GpuPick {
                                    chip: chip_index,
                                    input: input_index,
                                    vendor,
                                    placement
                                }
                            )
                        })
                        .min_by_key(|(rank, _)| *rank)
                })
                .min_by_key(|(rank, _)| *rank)
                .map(|(_, pick)| pick)
        }

        /// Reports whether a configured preference names this device.
        #[must_use]
        pub fn matches(
            preferred: &str,
            chip: &str,
            vendor: GpuVendor,
            placement: GpuPlacement
        ) -> bool {
            if preferred.is_empty() {
                return false;
            }

            let placement_name = match placement {
                GpuPlacement::Discrete => "discrete",
                GpuPlacement::Integrated => "integrated",
                GpuPlacement::Unknown => "unknown"
            };

            preferred == catalog::normalise_chip(chip)
                || preferred == vendor.as_str()
                || preferred == placement_name
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            /// Chip set of a machine, written down the way `sensors` prints it.
            struct Sensors {
                chips: Vec<(String, Vec<(String, f32)>, ChipFacts)>
            }

            impl Sensors {
                /// Builds a machine out of `("<chip> <input>", value)` pairs.
                fn from_labels(readings: &[(&str, f32)]) -> Self {
                    let mut chips: Vec<(String, Vec<(String, f32)>, ChipFacts)> = Vec::new();

                    for (label, value) in readings {
                        let (chip, input) = match label.split_once(' ') {
                            Some((chip, input)) => (chip, input),
                            None => (*label, "")
                        };

                        match chips.iter_mut().find(|(name, _, _)| name == chip) {
                            Some((_, inputs, _)) => inputs.push((input.to_owned(), *value)),
                            None => chips.push((
                                chip.to_owned(),
                                vec![(input.to_owned(), *value)],
                                ChipFacts::default()
                            ))
                        }
                    }

                    Self {
                        chips
                    }
                }

                /// Marks a chip as publishing the northbridge rail of an
                /// integrated part.
                fn integrated(mut self, chip: &str) -> Self {
                    self.mark(chip, |facts| facts.northbridge_voltage = true);
                    self
                }

                /// Marks a chip as owning a fan, the way a card does.
                fn with_fan(mut self, chip: &str) -> Self {
                    self.mark(chip, |facts| facts.fan = true);
                    self
                }

                /// Gives a chip an address on the bus.
                fn at(mut self, chip: &str, address: &'static str) -> Self {
                    self.mark(chip, move |facts| {
                        facts.on_root_complex =
                            ChipFacts::from_address(Some(address)).on_root_complex;
                    });
                    self
                }

                fn mark(&mut self, chip: &str, apply: impl Fn(&mut ChipFacts)) {
                    if let Some((_, _, facts)) =
                        self.chips.iter_mut().find(|(name, _, _)| name == chip)
                    {
                        apply(facts);
                    }
                }

                fn views(&self) -> (Vec<Vec<&str>>, Vec<ChipFacts>) {
                    let inputs = self
                        .chips
                        .iter()
                        .map(|(_, inputs, _)| {
                            inputs.iter().map(|(label, _)| label.as_str()).collect()
                        })
                        .collect();
                    let facts = self.chips.iter().map(|(_, _, facts)| *facts).collect();

                    (inputs, facts)
                }

                fn cpu(&self) -> Option<(String, f32)> {
                    let (inputs, facts) = self.views();
                    let views = self.chip_views(&inputs, &facts);

                    select_cpu(&views).map(|pick| self.reading(pick.chip, pick.input))
                }

                fn gpu(&self, preferred: Option<&str>) -> Option<(String, f32, GpuPlacement)> {
                    let (inputs, facts) = self.views();
                    let views = self.chip_views(&inputs, &facts);

                    select_gpu(&views, preferred).map(|pick| {
                        let (label, value) = self.reading(pick.chip, pick.input);

                        (label, value, pick.placement)
                    })
                }

                fn chip_views<'a>(
                    &'a self,
                    inputs: &'a [Vec<&'a str>],
                    facts: &'a [ChipFacts]
                ) -> Vec<ChipView<'a>> {
                    self.chips
                        .iter()
                        .enumerate()
                        .map(|(index, (chip, _, _))| ChipView {
                            chip:   chip.as_str(),
                            inputs: inputs[index].as_slice(),
                            facts:  facts[index]
                        })
                        .collect()
                }

                fn reading(&self, chip: usize, input: usize) -> (String, f32) {
                    let (name, inputs, _) = &self.chips[chip];
                    let (label, value) = &inputs[input];

                    (format!("{name} {label}").trim().to_owned(), *value)
                }
            }

            /// Desktop machine: AMD processor, AMD card, storage and network
            /// noise.
            fn amd_desktop() -> Sensors {
                Sensors::from_labels(&[
                    ("k10temp Tctl", 56.6),
                    ("k10temp Tccd1", 51.0),
                    ("amdgpu edge", 47.0),
                    ("amdgpu junction", 55.0),
                    ("amdgpu mem", 62.0),
                    ("nvme Composite", 39.9),
                    ("acpitz_0 temp1", 59.0),
                    ("r8169_0_c100:00 temp1", 48.5),
                    ("mt7925_phy0 temp1", 49.0)
                ])
                .with_fan("amdgpu")
            }

            /// Laptop with switchable graphics: Intel processor and graphics
            /// block, NVIDIA card awake.
            fn intel_laptop_with_card() -> Sensors {
                Sensors::from_labels(&[
                    ("coretemp Package id 0", 58.0),
                    ("coretemp Core 0", 71.0),
                    ("coretemp Core 1", 69.0),
                    ("i915 temp1", 45.0),
                    ("nvidia temp1", 66.0),
                    ("acpitz temp1", 47.0),
                    ("nvme Composite", 41.0)
                ])
                .at("i915", "0000:00:02.0")
            }

            #[test]
            fn the_processor_chip_beats_the_board_thermal_zone() {
                assert_eq!(amd_desktop().cpu(), Some(("k10temp Tctl".to_owned(), 56.6)));
            }

            #[test]
            fn the_package_reading_beats_a_single_core() {
                assert_eq!(
                    intel_laptop_with_card().cpu(),
                    Some(("coretemp Package id 0".to_owned(), 58.0))
                );
            }

            #[test]
            fn a_core_stands_in_when_the_chip_names_no_package() {
                let sensors =
                    Sensors::from_labels(&[("acpitz temp1", 80.0), ("k10temp Tccd1", 61.0)]);

                assert_eq!(sensors.cpu(), Some(("k10temp Tccd1".to_owned(), 61.0)));
            }

            #[test]
            fn the_thermal_zone_stands_in_when_no_processor_chip_reports() {
                let sensors = Sensors::from_labels(&[
                    ("amdgpu edge", 52.0),
                    ("nvme Composite", 42.8),
                    ("acpitz_0 temp1", 47.0)
                ]);

                assert_eq!(sensors.cpu(), Some(("acpitz_0 temp1".to_owned(), 47.0)));
            }

            #[test]
            fn storage_and_network_chips_are_never_the_processor_or_the_graphics() {
                let sensors = Sensors::from_labels(&[
                    ("nvme Composite", 42.8),
                    ("nvme Sensor 1", 40.8),
                    ("r8169_0_c100:00 temp1", 48.5),
                    ("mt7925_phy0 temp1", 49.0)
                ]);

                assert_eq!(sensors.cpu(), None);
                assert_eq!(sensors.gpu(None), None);
            }

            #[test]
            fn a_machine_without_sensors_reports_nothing() {
                let sensors = Sensors::from_labels(&[]);

                assert_eq!(sensors.cpu(), None);
                assert_eq!(sensors.gpu(None), None);
            }

            #[test]
            fn the_graphics_die_reading_wins_over_the_edge_and_the_memory() {
                let (label, value, placement) = amd_desktop().gpu(None).expect("graphics reading");

                assert_eq!((label.as_str(), value), ("amdgpu junction", 55.0));
                assert_eq!(placement, GpuPlacement::Discrete);
            }

            #[test]
            fn the_card_wins_over_the_block_inside_the_processor() {
                let (label, value, placement) = intel_laptop_with_card()
                    .gpu(None)
                    .expect("graphics reading");

                assert_eq!((label.as_str(), value), ("nvidia temp1", 66.0));
                assert_eq!(placement, GpuPlacement::Discrete);
            }

            #[test]
            fn the_block_inside_the_processor_is_reported_as_integrated_when_alone() {
                let sensors = Sensors::from_labels(&[
                    ("k10temp Tctl", 71.6),
                    ("amdgpu edge", 47.0),
                    ("acpitz_0 temp1", 62.0)
                ])
                .integrated("amdgpu");

                let (label, value, placement) = sensors.gpu(None).expect("graphics reading");

                assert_eq!((label.as_str(), value), ("amdgpu edge", 47.0));
                assert_eq!(placement, GpuPlacement::Integrated);
                assert_eq!(placement.tag(), Some("iGPU"));
            }

            #[test]
            fn a_sleeping_card_leaves_the_block_inside_the_processor_and_says_so() {
                let sensors = Sensors::from_labels(&[
                    ("coretemp Package id 0", 58.0),
                    ("i915 temp1", 45.0),
                    ("acpitz temp1", 47.0)
                ])
                .at("i915", "0000:00:02.0");

                let (label, value, placement) = sensors.gpu(None).expect("graphics reading");

                assert_eq!((label.as_str(), value), ("i915 temp1", 45.0));
                assert_eq!(placement, GpuPlacement::Integrated);
                assert_eq!(
                    placement.tag(),
                    Some("iGPU"),
                    "the reading of the block inside the processor is never shown as the card"
                );
            }

            #[test]
            fn an_intel_card_behind_a_bridge_is_not_taken_for_the_block_in_the_processor() {
                let sensors = Sensors::from_labels(&[("xe temp1", 61.0)]).at("xe", "0000:03:00.0");

                let (_, _, placement) = sensors.gpu(None).expect("graphics reading");

                assert_eq!(placement, GpuPlacement::Discrete);
                assert_eq!(placement.tag(), None);
            }

            #[test]
            fn a_machine_with_three_devices_reports_a_card_and_can_be_pinned_to_either() {
                let sensors = Sensors::from_labels(&[
                    ("i915 temp1", 45.0),
                    ("amdgpu junction", 58.0),
                    ("nvidia temp1", 66.0)
                ])
                .at("i915", "0000:00:02.0")
                .with_fan("amdgpu");

                let (label, _, placement) = sensors.gpu(None).expect("graphics reading");

                assert_eq!(placement, GpuPlacement::Discrete);
                assert!(label == "amdgpu junction" || label == "nvidia temp1");
                assert_eq!(
                    sensors.gpu(Some("nvidia")).map(|(label, _, _)| label),
                    Some("nvidia temp1".to_owned())
                );
                assert_eq!(
                    sensors.gpu(Some("integrated")).map(|(label, _, _)| label),
                    Some("i915 temp1".to_owned())
                );
            }

            #[test]
            fn two_cards_pick_the_one_the_configuration_names() {
                let sensors =
                    Sensors::from_labels(&[("amdgpu junction", 55.0), ("nvidia temp1", 66.0)])
                        .with_fan("amdgpu");

                assert_eq!(
                    sensors.gpu(Some("amd")).map(|(label, _, _)| label),
                    Some("amdgpu junction".to_owned())
                );
                assert_eq!(
                    sensors.gpu(Some("nvidia")).map(|(label, _, _)| label),
                    Some("nvidia temp1".to_owned())
                );
                assert_eq!(
                    sensors.gpu(Some("integrated")).map(|(label, _, _)| label),
                    Some("amdgpu junction".to_owned()),
                    "a preference that matches nothing must not hide the machine"
                );
            }

            #[test]
            fn a_second_processor_package_does_not_break_the_choice() {
                let sensors = Sensors::from_labels(&[
                    ("coretemp Package id 0", 58.0),
                    ("coretemp_1 Package id 1", 61.0)
                ]);

                assert_eq!(
                    sensors.cpu(),
                    Some(("coretemp Package id 0".to_owned(), 58.0))
                );
            }

            #[test]
            fn a_chip_with_a_numeric_suffix_keeps_its_family() {
                let sensors = Sensors::from_labels(&[("acpitz_1 temp1", 44.0)]);

                assert_eq!(sensors.cpu(), Some(("acpitz_1 temp1".to_owned(), 44.0)));
            }

            #[test]
            fn an_unlabelled_input_still_reports_when_it_is_all_the_chip_has() {
                let sensors = Sensors::from_labels(&[("nvidia temp1", 66.0)]);

                assert_eq!(
                    sensors.gpu(None).map(|(label, value, _)| (label, value)),
                    Some(("nvidia temp1".to_owned(), 66.0))
                );
            }
        }
    }
    pub mod utility {
        //! Optional vendor utilities, for the metrics the kernel keeps to
        //! itself.
        //!
        //! Everything the panel shows comes from the kernel first. A vendor
        //! utility is consulted only where the kernel publishes
        //! nothing, it is looked up once rather than on every refresh,
        //! it runs on a thread of its own so a slow or wedged program
        //! never delays the bar, and its absence is an ordinary state
        //! that costs nothing and is never logged as a fault.
        //!
        //! The AMD compute stack ships such a utility as well, and the panel
        //! does not run it: the graphics driver already publishes the
        //! temperature, the load and the video memory of every AMD part
        //! through the kernel, so spawning a process would buy nothing.
        //! Adding a vendor is one entry in [`UTILITIES`] together with
        //! the function that reads its output.

        use std::{
            io,
            path::PathBuf,
            process::Command,
            sync::{
                Arc, Mutex,
                atomic::{AtomicBool, Ordering}
            },
            thread,
            time::Duration
        };

        use log::debug;

        use super::catalog::GpuVendor;

        /// How long the panel waits between two runs of a vendor utility.
        ///
        /// A utility costs a process, and on a laptop it can keep a card from
        /// powering down, so it runs far more rarely than the bar
        /// refreshes and the bar shows the last answer in between.
        pub const UTILITY_INTERVAL: Duration = Duration::from_secs(30);

        /// Slice the poll waits in, so a dropped feed is noticed promptly.
        const STOP_CHECK: Duration = Duration::from_millis(250);

        /// Metrics a vendor utility reports for one device.
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
        pub struct UtilityMetrics {
            pub temperature:  Option<i32>,
            pub utilisation:  Option<u32>,
            pub memory_used:  Option<u64>,
            pub memory_total: Option<u64>
        }

        impl UtilityMetrics {
            /// Reports whether the utility answered with any usable number.
            #[must_use]
            pub fn is_empty(&self) -> bool {
                self.temperature.is_none()
                    && self.utilisation.is_none()
                    && self.memory_used.is_none()
                    && self.memory_total.is_none()
            }
        }

        /// A program the panel may ask for metrics the kernel does not publish.
        pub struct Utility {
            /// Program name, looked up on `PATH`.
            pub program: &'static str,
            pub args:    &'static [&'static str],
            /// Vendor whose devices the program reports on.
            pub vendor:  GpuVendor,
            /// Turns the standard output of the program into metrics.
            pub parse:   fn(&str) -> Option<UtilityMetrics>
        }

        /// Utilities the panel knows how to read.
        ///
        /// The NVIDIA driver publishes a monitoring chip only in its recent
        /// releases, and never publishes the load, so its query
        /// interface is the one way to show those numbers on an
        /// otherwise supported machine.
        pub const UTILITIES: [Utility; 1] = [Utility {
            program: "nvidia-smi",
            args:    &[
                "--query-gpu=temperature.gpu,utilization.gpu,memory.used,memory.total",
                "--format=csv,noheader,nounits"
            ],
            vendor:  GpuVendor::Nvidia,
            parse:   parse_query_row
        }];

        /// The utility that reports on a vendor, when the panel knows one.
        #[must_use]
        pub fn for_vendor(vendor: GpuVendor) -> Option<&'static Utility> {
            UTILITIES.iter().find(|utility| utility.vendor == vendor)
        }

        /// Path a program resolves to on `PATH`, when it is installed at all.
        #[must_use]
        pub fn on_path(program: &str) -> Option<PathBuf> {
            std::env::var_os("PATH").and_then(|paths| {
                std::env::split_paths(&paths)
                    .map(|directory| directory.join(program))
                    .find(|candidate| candidate.is_file())
            })
        }

        /// Runs a program and hands back what it wrote.
        pub trait Runner: Send + 'static {
            /// Runs the program, or fails the way the operating system did.
            fn run(&self, program: &str, args: &[&str]) -> io::Result<String>;
        }

        /// Runner that starts the program as a child process.
        #[derive(Debug, Default, Clone, Copy)]
        pub struct ProcessRunner;

        impl Runner for ProcessRunner {
            fn run(&self, program: &str, args: &[&str]) -> io::Result<String> {
                let output = Command::new(program)
                    .args(args)
                    .stdin(std::process::Stdio::null())
                    .output()?;

                if !output.status.success() {
                    return Err(io::Error::other(format!(
                        "{program} exited with {}",
                        output.status
                    )));
                }

                Ok(String::from_utf8_lossy(&output.stdout).into_owned())
            }
        }

        /// Latest answer of a vendor utility, refreshed on a thread of its own.
        #[derive(Debug)]
        pub struct Feed {
            vendor: GpuVendor,
            latest: Arc<Mutex<Option<UtilityMetrics>>>,
            stop:   Arc<AtomicBool>
        }

        impl Feed {
            /// Starts polling a utility.
            ///
            /// `gate` decides whether the device may be queried at all, which
            /// is how a sleeping card is left alone instead of
            /// being woken up for a reading.
            pub fn spawn<R, G>(
                utility: &'static Utility,
                runner: R,
                gate: G,
                period: Duration
            ) -> Self
            where
                R: Runner,
                G: Fn() -> bool + Send + 'static
            {
                let latest = Arc::new(Mutex::new(None));
                let stop = Arc::new(AtomicBool::new(false));

                thread::spawn({
                    let latest = Arc::clone(&latest);
                    let stop = Arc::clone(&stop);

                    move || {
                        let mut reported = false;

                        while !stop.load(Ordering::Relaxed) {
                            let sample = poll(utility, &runner, &gate, &mut reported);
                            store(&latest, sample);

                            let mut waited = Duration::ZERO;
                            while waited < period && !stop.load(Ordering::Relaxed) {
                                thread::sleep(STOP_CHECK.min(period - waited));
                                waited += STOP_CHECK;
                            }
                        }
                    }
                });

                Self {
                    vendor: utility.vendor,
                    latest,
                    stop
                }
            }

            /// Vendor the feed reports on.
            #[must_use]
            pub fn vendor(&self) -> GpuVendor {
                self.vendor
            }

            /// Latest metrics, or [`None`] while the device reports none.
            #[must_use]
            pub fn latest(&self) -> Option<UtilityMetrics> {
                self.latest.lock().ok().and_then(|latest| *latest)
            }
        }

        impl Drop for Feed {
            fn drop(&mut self) {
                self.stop.store(true, Ordering::Relaxed);
            }
        }

        fn store(latest: &Mutex<Option<UtilityMetrics>>, sample: Option<UtilityMetrics>) {
            if let Ok(mut slot) = latest.lock() {
                *slot = sample;
            }
        }

        /// One run of a utility.
        ///
        /// A closed gate and a program that failed both mean the same to the
        /// bar: no reading right now. The failure is written to the log
        /// once per feed, because a device that is simply absent must
        /// not fill the journal every refresh.
        fn poll<R, G>(
            utility: &Utility,
            runner: &R,
            gate: &G,
            reported: &mut bool
        ) -> Option<UtilityMetrics>
        where
            R: Runner,
            G: Fn() -> bool
        {
            if !gate() {
                return None;
            }

            match runner.run(utility.program, utility.args) {
                Ok(output) => {
                    *reported = false;

                    (utility.parse)(&output).filter(|metrics| !metrics.is_empty())
                }
                Err(error) => {
                    if !*reported {
                        debug!("{} reported no metrics: {error}", utility.program);
                        *reported = true;
                    }

                    None
                }
            }
        }

        /// Reads one comma separated row of `temperature, load, memory used,
        /// memory installed`.
        ///
        /// Only the first row is read: a machine with two cards answers with
        /// one row each, and the panel shows the card it selected
        /// rather than a sum. Fields the driver cannot answer arrive as
        /// `[N/A]` and stay empty instead of being shown as a zero.
        fn parse_query_row(output: &str) -> Option<UtilityMetrics> {
            let row = output.lines().find(|line| !line.trim().is_empty())?;
            let mut fields = row.split(',').map(str::trim);

            let temperature = fields.next().and_then(|value| value.parse().ok());
            let utilisation = fields
                .next()
                .and_then(|value| value.parse::<u32>().ok())
                .map(|value| value.min(100));
            let memory_used = fields.next().and_then(parse_mebibytes);
            let memory_total = fields.next().and_then(parse_mebibytes);

            Some(UtilityMetrics {
                temperature,
                utilisation,
                memory_used,
                memory_total
            })
        }

        /// Turns a count of mebibytes into bytes, the unit the module carries.
        fn parse_mebibytes(value: &str) -> Option<u64> {
            value.parse::<u64>().ok().map(|value| value * 1024 * 1024)
        }

        #[cfg(test)]
        mod tests {
            use std::sync::atomic::AtomicUsize;

            use super::*;

            struct Scripted {
                output: &'static str,
                runs:   Arc<AtomicUsize>
            }

            impl Runner for Scripted {
                fn run(&self, _: &str, _: &[&str]) -> io::Result<String> {
                    self.runs.fetch_add(1, Ordering::Relaxed);

                    Ok(self.output.to_owned())
                }
            }

            struct Failing;

            impl Runner for Failing {
                fn run(&self, _: &str, _: &[&str]) -> io::Result<String> {
                    Err(io::Error::new(io::ErrorKind::NotFound, "no such program"))
                }
            }

            fn nvidia() -> &'static Utility {
                for_vendor(GpuVendor::Nvidia).expect("a utility for the vendor")
            }

            #[test]
            fn a_query_row_becomes_metrics() {
                let metrics = parse_query_row("66, 15, 1234, 8192\n").expect("metrics");

                assert_eq!(metrics.temperature, Some(66));
                assert_eq!(metrics.utilisation, Some(15));
                assert_eq!(metrics.memory_used, Some(1234 * 1024 * 1024));
                assert_eq!(metrics.memory_total, Some(8192 * 1024 * 1024));
            }

            #[test]
            fn only_the_first_card_of_the_answer_is_read() {
                let metrics =
                    parse_query_row("66, 15, 1234, 8192\n71, 40, 512, 8192\n").expect("metrics");

                assert_eq!(metrics.temperature, Some(66));
            }

            #[test]
            fn a_field_the_driver_cannot_answer_stays_empty() {
                let metrics = parse_query_row("[N/A], 15, [N/A], 8192").expect("metrics");

                assert_eq!(metrics.temperature, None);
                assert_eq!(metrics.utilisation, Some(15));
                assert_eq!(metrics.memory_used, None);
                assert_eq!(metrics.memory_total, Some(8192 * 1024 * 1024));
            }

            #[test]
            fn an_empty_answer_reports_nothing() {
                assert_eq!(parse_query_row(""), None);
                assert!(
                    parse_query_row("[N/A], [N/A], [N/A], [N/A]")
                        .expect("metrics")
                        .is_empty()
                );
            }

            #[test]
            fn a_closed_gate_leaves_the_device_alone() {
                let runs = Arc::new(AtomicUsize::new(0));
                let runner = Scripted {
                    output: "66, 15, 1234, 8192",
                    runs:   Arc::clone(&runs)
                };
                let mut reported = false;

                assert_eq!(poll(nvidia(), &runner, &|| false, &mut reported), None);
                assert_eq!(runs.load(Ordering::Relaxed), 0);
            }

            #[test]
            fn an_open_gate_reads_the_device() {
                let runs = Arc::new(AtomicUsize::new(0));
                let runner = Scripted {
                    output: "66, 15, 1234, 8192",
                    runs:   Arc::clone(&runs)
                };
                let mut reported = false;

                let metrics = poll(nvidia(), &runner, &|| true, &mut reported).expect("metrics");

                assert_eq!(metrics.temperature, Some(66));
                assert_eq!(runs.load(Ordering::Relaxed), 1);
            }

            #[test]
            fn a_missing_program_is_reported_once_and_yields_nothing() {
                let mut reported = false;

                assert_eq!(poll(nvidia(), &Failing, &|| true, &mut reported), None);
                assert!(reported, "the first failure is written to the log");

                assert_eq!(poll(nvidia(), &Failing, &|| true, &mut reported), None);
                assert!(reported, "the next failures stay quiet");
            }

            #[test]
            fn a_feed_publishes_what_the_utility_answered() {
                let runs = Arc::new(AtomicUsize::new(0));
                let feed = Feed::spawn(
                    nvidia(),
                    Scripted {
                        output: "66, 15, 1234, 8192",
                        runs:   Arc::clone(&runs)
                    },
                    || true,
                    Duration::from_millis(20)
                );

                let mut metrics = None;
                for _ in 0..200 {
                    metrics = feed.latest();
                    if metrics.is_some() {
                        break;
                    }

                    thread::sleep(Duration::from_millis(5));
                }

                assert_eq!(feed.vendor(), GpuVendor::Nvidia);
                assert_eq!(metrics.expect("metrics").temperature, Some(66));
            }
        }
    }

    use std::{
        path::{Path, PathBuf},
        time::{Duration, Instant}
    };

    use log::warn;

    pub use self::catalog::{GpuPlacement, GpuVendor};
    use self::{
        drm::Card,
        selection::{ChipView, select_cpu, select_gpu},
        utility::{Feed, ProcessRunner, UTILITY_INTERVAL}
    };

    /// How long the panel keeps a sensor set before rebuilding it.
    ///
    /// Hardware comes and goes while the bar runs, and a walk of two small
    /// sysfs trees is cheap, but not cheap enough to repeat every few
    /// seconds for a set that almost never changes.
    pub const DISCOVERY_INTERVAL: Duration = Duration::from_secs(30);

    /// Temperature of the processor and everything known about the graphics.
    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    pub struct Readings {
        /// Processor temperature in whole degrees Celsius.
        pub cpu: Option<i32>,
        pub gpu: Option<GpuReadings>
    }

    /// What the panel can say about the graphics processor it watches.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GpuReadings {
        /// Driver behind the device, such as `amdgpu` or `nvidia`.
        pub name:         String,
        /// Chip and input the temperature is taken from, for the menu.
        pub source:       Option<String>,
        pub vendor:       GpuVendor,
        pub placement:    GpuPlacement,
        /// Temperature in whole degrees Celsius.
        pub temperature:  Option<i32>,
        /// Share of the device that is busy, in percent.
        pub utilisation:  Option<u32>,
        pub memory_used:  Option<u64>,
        pub memory_total: Option<u64>
    }

    impl GpuReadings {
        /// Short tag the bar puts in front of the reading, when one is needed.
        #[must_use]
        pub fn tag(&self) -> Option<&'static str> {
            self.placement.tag()
        }

        /// Reports whether the device answered with anything worth drawing.
        #[must_use]
        fn is_empty(&self) -> bool {
            self.temperature.is_none() && self.utilisation.is_none()
        }
    }

    /// Temperature input the panel settled on.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Input {
        label: String,
        chip:  String,
        path:  PathBuf
    }

    impl Input {
        fn describe(&self) -> String {
            if self.label.is_empty() {
                self.chip.clone()
            } else {
                format!("{} {}", self.chip, self.label)
            }
        }
    }

    /// Graphics device the panel settled on.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Gpu {
        name:      String,
        vendor:    GpuVendor,
        placement: GpuPlacement,
        input:     Option<Input>,
        card:      Option<Card>
    }

    /// Sensors of the machine, discovered once and read from then on.
    #[derive(Debug)]
    pub struct HardwareSensors {
        hwmon_root:    PathBuf,
        drm_root:      PathBuf,
        preferred_gpu: Option<String>,
        discovered_at: Option<Instant>,
        cpu:           Option<Input>,
        gpu:           Option<Gpu>,
        utility:       Option<Feed>,
        buffer:        String,
        reported:      bool
    }

    impl Default for HardwareSensors {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HardwareSensors {
        /// Sensors of the machine the panel runs on.
        #[must_use]
        pub fn new() -> Self {
            Self::rooted(Path::new(hwmon::DEFAULT_ROOT), Path::new(drm::DEFAULT_ROOT))
        }

        /// Sensors published under the given subsystem roots.
        #[must_use]
        pub fn rooted(hwmon_root: &Path, drm_root: &Path) -> Self {
            Self {
                hwmon_root:    hwmon_root.to_path_buf(),
                drm_root:      drm_root.to_path_buf(),
                preferred_gpu: None,
                discovered_at: None,
                cpu:           None,
                gpu:           None,
                utility:       None,
                buffer:        String::with_capacity(32),
                reported:      false
            }
        }

        /// Pins the graphics device the panel reports on.
        ///
        /// The entry names a vendor, a driver or a placement; one that matches
        /// nothing on this machine is ignored, so a configuration written for
        /// another machine never leaves this one without a reading.
        pub fn prefer_gpu(&mut self, preferred: Option<&str>) {
            let preferred = preferred
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_owned);

            if preferred != self.preferred_gpu {
                self.preferred_gpu = preferred;
                self.discovered_at = None;
            }
        }

        /// Latest readings, rebuilding the sensor set when it is due.
        pub fn read(&mut self) -> Readings {
            if self
                .discovered_at
                .is_none_or(|discovered| discovered.elapsed() >= DISCOVERY_INTERVAL)
            {
                self.discover();
            }

            let mut state = ReadState {
                failed:   false,
                reported: &mut self.reported
            };
            let cpu = read_input(self.cpu.as_ref(), &mut self.buffer, &mut state);

            let gpu = self.gpu.as_ref().map(|gpu| {
                let temperature = read_input(gpu.input.as_ref(), &mut self.buffer, &mut state);
                let awake = gpu.card.as_ref().is_none_or(|card| !card.is_asleep());
                let utilisation = awake
                    .then(|| {
                        gpu.card
                            .as_ref()
                            .and_then(|card| card.utilisation(&mut self.buffer))
                    })
                    .flatten();
                let memory = awake
                    .then(|| {
                        gpu.card
                            .as_ref()
                            .and_then(|card| card.memory(&mut self.buffer))
                    })
                    .flatten();

                GpuReadings {
                    name: gpu.name.clone(),
                    source: gpu.input.as_ref().map(Input::describe),
                    vendor: gpu.vendor,
                    placement: gpu.placement,
                    temperature,
                    utilisation,
                    memory_used: memory.map(|(used, _)| used),
                    memory_total: memory.map(|(_, total)| total)
                }
            });

            let failed = state.failed;
            let gpu = self
                .complete_with_utility(gpu)
                .filter(|gpu| !gpu.is_empty());

            if failed {
                self.discovered_at = None;
            } else {
                self.reported = false;
            }

            Readings {
                cpu,
                gpu
            }
        }

        /// Fills in what the kernel does not publish from the vendor utility.
        fn complete_with_utility(&self, gpu: Option<GpuReadings>) -> Option<GpuReadings> {
            let mut gpu = gpu?;
            let Some(metrics) = self
                .utility
                .as_ref()
                .filter(|feed| feed.vendor() == gpu.vendor)
                .and_then(Feed::latest)
            else {
                return Some(gpu);
            };

            gpu.temperature = gpu.temperature.or(metrics.temperature);
            gpu.utilisation = gpu.utilisation.or(metrics.utilisation);
            gpu.memory_used = gpu.memory_used.or(metrics.memory_used);
            gpu.memory_total = gpu.memory_total.or(metrics.memory_total);

            Some(gpu)
        }

        /// Rebuilds the sensor set out of what the kernel publishes right now.
        fn discover(&mut self) {
            let chips = hwmon::scan(&self.hwmon_root);
            let cards = drm::scan(&self.drm_root);
            let labels: Vec<Vec<&str>> = chips
                .iter()
                .map(|chip| {
                    chip.inputs
                        .iter()
                        .map(|input| input.label.as_str())
                        .collect()
                })
                .collect();
            let views: Vec<ChipView<'_>> = chips
                .iter()
                .enumerate()
                .map(|(index, chip)| ChipView {
                    chip:   chip.name.as_str(),
                    inputs: labels[index].as_slice(),
                    facts:  chip.facts
                })
                .collect();

            let cpu = select_cpu(&views).map(|pick| Input {
                label: chips[pick.chip].inputs[pick.input].label.clone(),
                chip:  chips[pick.chip].name.clone(),
                path:  chips[pick.chip].inputs[pick.input].path.clone()
            });

            let gpu = select_gpu(&views, self.preferred_gpu.as_deref())
                .map(|pick| {
                    let chip = &chips[pick.chip];
                    let input = &chip.inputs[pick.input];

                    Gpu {
                        name:      chip.name.clone(),
                        vendor:    pick.vendor,
                        placement: pick.placement,
                        input:     Some(Input {
                            label: input.label.clone(),
                            chip:  chip.name.clone(),
                            path:  input.path.clone()
                        }),
                        card:      pair_card(&cards, chip.device.as_deref())
                    }
                })
                .or_else(|| card_only_gpu(&cards));

            self.cpu = cpu;
            self.gpu = gpu;
            self.utility = self.utility_feed();
            self.discovered_at = Some(Instant::now());
        }

        /// Starts, keeps or stops the vendor utility behind the selected
        /// device.
        ///
        /// It is started only where the kernel publishes neither a temperature
        /// nor a load for the device, and only when the program is
        /// installed, so a machine the kernel covers never spawns a
        /// process.
        fn utility_feed(&mut self) -> Option<Feed> {
            let gpu = self.gpu.as_ref()?;
            let covered =
                gpu.input.is_some() && gpu.card.as_ref().is_some_and(|card| card.busy.is_some());

            if covered {
                return None;
            }

            if let Some(feed) = self.utility.take()
                && feed.vendor() == gpu.vendor
            {
                return Some(feed);
            }

            let utility = utility::for_vendor(gpu.vendor)?;
            utility::on_path(utility.program)?;

            let card = gpu.card.clone();

            Some(Feed::spawn(
                utility,
                ProcessRunner,
                move || card.as_ref().is_none_or(|card| !card.is_asleep()),
                UTILITY_INTERVAL
            ))
        }
    }

    /// State one pass over the chosen inputs leaves behind.
    struct ReadState<'a> {
        /// An input that was there at discovery no longer reads.
        failed:   bool,
        /// The failure has already been written to the log.
        reported: &'a mut bool
    }

    /// Reading behind an input, noting a failure for the caller.
    ///
    /// A sensor that stops answering is written to the log once and then left
    /// alone: the panel reads every few seconds, and a machine whose card was
    /// unplugged would otherwise fill the journal with the same line. The next
    /// successful pass clears the mark, so a fault that comes back is reported
    /// again.
    fn read_input(
        input: Option<&Input>,
        buffer: &mut String,
        state: &mut ReadState<'_>
    ) -> Option<i32> {
        let input = input?;

        match hwmon::read_temperature(&input.path, buffer) {
            Ok(temperature) => Some(temperature),
            Err(error) => {
                if !*state.reported {
                    warn!("sensor {} stopped reporting: {error}", input.describe());
                    *state.reported = true;
                }

                state.failed = true;

                None
            }
        }
    }

    /// Card the kernel links to the same device as the chosen chip.
    fn pair_card(cards: &[Card], device: Option<&Path>) -> Option<Card> {
        let device = device?;

        cards
            .iter()
            .find(|card| card.device.as_deref() == Some(device))
            .cloned()
    }

    /// Graphics device known from the rendering subsystem alone.
    ///
    /// A driver that registers no monitoring chip still publishes a device,
    /// which is what a machine reports while its vendor module lacks
    /// monitoring support or while the reading comes from a vendor utility
    /// instead.
    fn card_only_gpu(cards: &[Card]) -> Option<Gpu> {
        let card = cards
            .iter()
            .filter(|card| card.vendor.is_some())
            .min_by_key(|card| (u8::from(card.busy.is_none()), card.driver.clone()))?;
        let vendor = card.vendor?;
        let facts = selection::ChipFacts::from_address(
            card.device
                .as_deref()
                .and_then(Path::file_name)
                .and_then(|address| address.to_str())
        );

        Some(Gpu {
            name: card
                .driver
                .clone()
                .unwrap_or_else(|| vendor.as_str().to_owned()),
            vendor,
            placement: selection::placement(vendor, facts),
            input: None,
            card: Some(card.clone())
        })
    }

    #[cfg(test)]
    mod tests {
        use std::fs;

        use tempfile::TempDir;

        use super::*;

        /// Sysfs of a machine, written down attribute by attribute.
        struct Machine {
            hwmon:   TempDir,
            drm:     TempDir,
            devices: TempDir
        }

        impl Machine {
            fn new() -> Self {
                Self {
                    hwmon:   TempDir::new().expect("monitoring root"),
                    drm:     TempDir::new().expect("rendering root"),
                    devices: TempDir::new().expect("device root")
                }
            }

            /// Directory standing for one piece of hardware on the bus.
            fn device(&self, slot: &str) -> PathBuf {
                let device = self.devices.path().join(slot);
                fs::create_dir_all(&device).expect("device directory");

                device
            }

            fn chip(self, index: usize, name: &str, inputs: &[(&str, &str)]) -> Self {
                let directory = self.hwmon.path().join(format!("hwmon{index}"));
                fs::create_dir_all(&directory).expect("chip directory");
                fs::write(directory.join("name"), name).expect("chip name");

                for (position, (label, value)) in inputs.iter().enumerate() {
                    let input = position + 1;
                    fs::write(directory.join(format!("temp{input}_input")), value)
                        .expect("reading");

                    if !label.is_empty() {
                        fs::write(directory.join(format!("temp{input}_label")), label)
                            .expect("label");
                    }
                }

                self
            }

            fn integrated(self, index: usize) -> Self {
                let directory = self.hwmon.path().join(format!("hwmon{index}"));
                fs::write(directory.join("in1_label"), "vddnb").expect("rail");

                self
            }

            fn card(self, index: usize, vendor: &str, busy: Option<&str>) -> Self {
                let device = self.drm.path().join(format!("card{index}")).join("device");
                fs::create_dir_all(&device).expect("card directory");
                fs::write(device.join("vendor"), vendor).expect("vendor");

                if let Some(busy) = busy {
                    fs::write(device.join("gpu_busy_percent"), busy).expect("load");
                    fs::write(device.join("mem_info_vram_used"), "1073741824").expect("used");
                    fs::write(device.join("mem_info_vram_total"), "8589934592").expect("total");
                }

                self
            }

            /// Attaches a chip and a card to the same piece of hardware, the
            /// way the kernel links both subsystems to one device.
            fn attach(self, slot: &str, chip: usize, card: usize) -> Self {
                let device = self.device(slot);
                let card_device = self.drm.path().join(format!("card{card}")).join("device");
                fs::write(device.join("vendor"), "0x1002").expect("vendor");

                for attribute in [
                    "vendor",
                    "gpu_busy_percent",
                    "mem_info_vram_used",
                    "mem_info_vram_total"
                ] {
                    let source = card_device.join(attribute);

                    if source.exists() {
                        fs::rename(&source, device.join(attribute)).expect("move attribute");
                    }
                }

                fs::remove_dir_all(&card_device).expect("replace card device");
                std::os::unix::fs::symlink(&device, &card_device).expect("card link");
                std::os::unix::fs::symlink(
                    &device,
                    self.hwmon
                        .path()
                        .join(format!("hwmon{chip}"))
                        .join("device")
                )
                .expect("chip link");

                self
            }

            fn sensors(&self) -> HardwareSensors {
                HardwareSensors::rooted(self.hwmon.path(), self.drm.path())
            }
        }

        #[test]
        fn a_machine_without_sensors_reports_nothing() {
            let machine = Machine::new();
            let readings = machine.sensors().read();

            assert_eq!(readings, Readings::default());
        }

        #[test]
        fn the_processor_and_the_graphics_are_two_separate_readings() {
            let machine = Machine::new()
                .chip(0, "acpitz_0", &[("", "62000")])
                .chip(1, "k10temp", &[("Tctl", "71625")])
                .chip(2, "amdgpu", &[("edge", "47000")])
                .integrated(2)
                .chip(3, "nvme", &[("Composite", "39850")]);

            let readings = machine.sensors().read();
            let gpu = readings.gpu.expect("graphics readings");

            assert_eq!(readings.cpu, Some(71));
            assert_eq!(gpu.temperature, Some(47));
            assert_eq!(gpu.placement, GpuPlacement::Integrated);
            assert_eq!(gpu.tag(), Some("iGPU"));
            assert_eq!(gpu.source.as_deref(), Some("amdgpu edge"));
        }

        #[test]
        fn load_and_memory_join_the_temperature_of_the_same_device() {
            let machine = Machine::new()
                .chip(0, "amdgpu", &[("edge", "47000")])
                .card(1, "0x1002", Some("11"))
                .attach("0000:c5:00.0", 0, 1);

            let gpu = machine.sensors().read().gpu.expect("graphics readings");

            assert_eq!(gpu.temperature, Some(47));
            assert_eq!(gpu.utilisation, Some(11));
            assert_eq!(gpu.memory_used, Some(1024 * 1024 * 1024));
            assert_eq!(gpu.memory_total, Some(8 * 1024 * 1024 * 1024));
        }

        #[test]
        fn a_second_card_does_not_lend_its_load_to_another_device() {
            let machine = Machine::new().chip(0, "amdgpu", &[("edge", "47000")]).card(
                1,
                "0x1002",
                Some("11")
            );

            let gpu = machine.sensors().read().gpu.expect("graphics readings");

            assert_eq!(gpu.temperature, Some(47));
            assert_eq!(gpu.utilisation, None);
        }

        #[test]
        fn a_card_without_a_monitoring_chip_still_reports_its_load() {
            let machine = Machine::new()
                .chip(0, "coretemp", &[("Package id 0", "58000")])
                .card(0, "0x10de", Some("42"));

            let readings = machine.sensors().read();
            let gpu = readings.gpu.expect("graphics readings");

            assert_eq!(readings.cpu, Some(58));
            assert_eq!(gpu.temperature, None);
            assert_eq!(gpu.utilisation, Some(42));
            assert_eq!(gpu.vendor, GpuVendor::Nvidia);
            assert_eq!(gpu.placement, GpuPlacement::Discrete);
        }

        #[test]
        fn a_sleeping_card_reports_no_load() {
            let machine = Machine::new().card(0, "0x10de", Some("42"));
            let device = machine.drm.path().join("card0/device/power");
            fs::create_dir_all(&device).expect("power directory");
            fs::write(device.join("runtime_status"), "suspended").expect("status");

            assert_eq!(machine.sensors().read().gpu, None);
        }

        #[test]
        fn a_disappearing_sensor_is_rediscovered_rather_than_repeated() {
            let machine = Machine::new().chip(0, "k10temp", &[("Tctl", "56600")]);
            let mut sensors = machine.sensors();

            assert_eq!(sensors.read().cpu, Some(56));

            fs::remove_dir_all(machine.hwmon.path().join("hwmon0")).expect("remove chip");

            assert_eq!(sensors.read().cpu, None);
            assert!(
                sensors.discovered_at.is_none(),
                "a failed read forces a rescan"
            );
            assert_eq!(sensors.read().cpu, None);
        }

        #[test]
        fn a_pinned_vendor_wins_over_the_better_placement() {
            let machine = Machine::new()
                .chip(0, "amdgpu", &[("edge", "47000")])
                .integrated(0)
                .chip(1, "nvidia", &[("", "66000")]);

            let mut sensors = machine.sensors();
            assert_eq!(
                sensors.read().gpu.expect("graphics readings").vendor,
                GpuVendor::Nvidia
            );

            sensors.prefer_gpu(Some("amd"));
            let gpu = sensors.read().gpu.expect("graphics readings");

            assert_eq!(gpu.vendor, GpuVendor::Amd);
            assert_eq!(gpu.temperature, Some(47));
        }

        #[test]
        fn a_processor_thermal_zone_is_never_shown_as_graphics() {
            let machine = Machine::new().chip(0, "cpu_thermal", &[("", "51000")]);
            let readings = machine.sensors().read();

            assert_eq!(readings.cpu, Some(51));
            assert_eq!(readings.gpu, None);
        }
    }
}
mod view {
    use iced::{
        Alignment, Element, Theme,
        widget::{Row, container, row}
    };

    use super::{Message, data::SystemInfoData, indicators, sensors::GpuReadings};
    use crate::{
        components::{
            icons::{IconTheme, Icons, icon},
            scale,
            text::text
        },
        config::{Appearance, MemoryFormat, SystemIndicator, SystemModuleConfig},
        menu::MenuType,
        modules::OnModulePress
    };

    /// Value of an indicator paired with the thresholds coloring it.
    #[derive(Debug, Clone, Copy)]
    struct Thresholds<V> {
        value: V,
        warn:  V,
        alert: V
    }

    impl<V> Thresholds<V> {
        fn new(value: V, warn: V, alert: V) -> Self {
            Self {
                value,
                warn,
                alert
            }
        }
    }

    /// Builds the text of an indicator out of an optional prefix, a value and
    /// its unit.
    fn indicator_label(prefix: Option<&str>, value: impl std::fmt::Display, unit: &str) -> String {
        match prefix {
            Some(prefix) => format!("{prefix} {value}{unit}"),
            None => format!("{value}{unit}")
        }
    }

    /// Amount of bytes rendered as gibibytes with a single decimal.
    ///
    /// The divisor is binary, so the unit next to the number has to be the
    /// binary one: eight gibibytes shown as `8.0GB` overstated every
    /// readout by seven percent against the unit it named.
    pub(crate) fn gigabytes(bytes: u64) -> String {
        format!("{:.1}", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }

    /// A pool stated as the amount in use against the amount there is.
    pub(crate) fn used_of_total(used: u64, total: u64) -> String {
        format!("{} / {} GiB", gigabytes(used), gigabytes(total))
    }

    /// Renders a memory readout in the format the active index selects.
    fn memory_label(format: MemoryFormat, prefix: Option<&str>, usage: u32, used: u64) -> String {
        match format {
            MemoryFormat::Percentage => indicator_label(prefix, usage, "%"),
            MemoryFormat::Bytes => indicator_label(prefix, gigabytes(used), "GiB")
        }
    }

    fn indicator_info_element<V>(
        icons: &IconTheme,
        info_icon: Icons,
        label: String,
        thresholds: Option<Thresholds<V>>,
        icon_label_gap: f32
    ) -> Element<'static, Message>
    where
        V: PartialOrd + Copy + 'static
    {
        let content = container(row!(icon(icons, info_icon), text(label)).spacing(icon_label_gap));

        if let Some(thresholds) = thresholds {
            content
                .style(move |theme: &Theme| container::Style {
                    text_color: if thresholds.value > thresholds.warn
                        && thresholds.value < thresholds.alert
                    {
                        Some(theme.palette().warning)
                    } else if thresholds.value >= thresholds.alert {
                        Some(theme.palette().danger)
                    } else {
                        None
                    },
                    ..Default::default()
                })
                .into()
        } else {
            content.into()
        }
    }

    /// Name of a graphics device as the menu spells it out.
    ///
    /// The placement is spelled out rather than abbreviated, so a machine with
    /// switchable graphics says which of its two devices the bar is watching.
    pub(crate) fn gpu_title(gpu: &GpuReadings) -> String {
        let placement = match gpu.tag() {
            Some(_) => "Integrated graphics",
            None => "Graphics"
        };

        match gpu.source.as_deref() {
            Some(source) => format!("{placement} ({source})"),
            None => format!("{placement} ({})", gpu.name)
        }
    }

    /// A transfer rate, handed in as kilobytes per second, spelled out.
    ///
    /// Above a thousand the rate reads in megabytes with one decimal: the
    /// integer division it replaced showed `1 MB/s` for anything up to
    /// `1999 KB/s`, understating a rate by up to half.
    pub(crate) fn format_speed(speed: u32) -> String {
        if speed >= 1000 {
            format!("{:.1} MB/s", f64::from(speed) / 1000.0)
        } else {
            format!("{speed} KB/s")
        }
    }

    /// Bar readout of one indicator, or nothing while this machine cannot
    /// draw it.
    ///
    /// The standalone processor and memory modules draw single readouts out
    /// of the same sample the combined module renders, so the one spelling of
    /// every readout lives here and the thin entries cannot drift from it.
    pub fn single_indicator<M>(
        indicator: &SystemIndicator,
        data: &SystemInfoData,
        config: &SystemModuleConfig,
        memory_format: MemoryFormat,
        appearance: &Appearance,
        icons: &IconTheme
    ) -> Option<Element<'static, M>>
    where
        M: 'static + From<Message>
    {
        let icon_label_gap = appearance.icon_label_gap();

        let element: Option<Element<'static, Message>> = match indicator {
            SystemIndicator::Cpu => Some(indicator_info_element(
                icons,
                Icons::Cpu,
                indicator_label(None, data.cpu_usage, "%"),
                Some(Thresholds::new(
                    data.cpu_usage,
                    config.cpu.warn_threshold,
                    config.cpu.alert_threshold
                )),
                icon_label_gap
            )),
            SystemIndicator::Memory => Some(indicator_info_element(
                icons,
                Icons::Mem,
                memory_label(memory_format, None, data.memory_usage, data.memory_used),
                Some(Thresholds::new(
                    data.memory_usage,
                    config.memory.warn_threshold,
                    config.memory.alert_threshold
                )),
                icon_label_gap
            )),
            SystemIndicator::MemorySwap => Some(indicator_info_element(
                icons,
                Icons::Mem,
                memory_label(
                    memory_format,
                    Some("swap"),
                    data.memory_swap_usage,
                    data.memory_swap_used
                ),
                Some(Thresholds::new(
                    data.memory_swap_usage,
                    config.memory.warn_threshold,
                    config.memory.alert_threshold
                )),
                icon_label_gap
            )),
            SystemIndicator::CpuTemperature => data.cpu_temperature.map(|temperature| {
                indicator_info_element(
                    icons,
                    Icons::Temp,
                    indicator_label(None, temperature, "°C"),
                    Some(Thresholds::new(
                        temperature,
                        config.temperature.warn_threshold,
                        config.temperature.alert_threshold
                    )),
                    icon_label_gap
                )
            }),
            SystemIndicator::GpuTemperature => data.gpu.as_ref().and_then(|gpu| {
                gpu.temperature.map(|temperature| {
                    indicator_info_element(
                        icons,
                        Icons::Gpu,
                        indicator_label(gpu.tag(), temperature, "°C"),
                        Some(Thresholds::new(
                            temperature,
                            config.gpu.warn_threshold,
                            config.gpu.alert_threshold
                        )),
                        icon_label_gap
                    )
                })
            }),
            SystemIndicator::GpuUsage => data.gpu.as_ref().and_then(|gpu| {
                gpu.utilisation.map(|usage| {
                    indicator_info_element(
                        icons,
                        Icons::Accelerator,
                        indicator_label(gpu.tag(), usage, "%"),
                        Some(Thresholds::new(
                            usage,
                            config.gpu.usage_warn_threshold,
                            config.gpu.usage_alert_threshold
                        )),
                        icon_label_gap
                    )
                })
            }),
            SystemIndicator::Disk(mount) => data.disks.iter().find_map(|disk| {
                if disk.mount == mount.as_str() {
                    Some(indicator_info_element(
                        icons,
                        Icons::Drive,
                        indicator_label(Some(disk.mount.as_str()), disk.usage_percent, "%"),
                        Some(Thresholds::new(
                            disk.usage_percent,
                            config.disk.warn_threshold,
                            config.disk.alert_threshold
                        )),
                        icon_label_gap
                    ))
                } else {
                    None
                }
            }),
            SystemIndicator::IpAddress => data.network.as_ref().map(|network| {
                let ip = network.ip.clone();
                container(row!(icon(icons, Icons::IpAddress), text(ip)).spacing(icon_label_gap))
                    .into()
            }),
            SystemIndicator::DownloadSpeed => data.network.as_ref().map(|network| {
                indicator_info_element::<u32>(
                    icons,
                    Icons::DownloadSpeed,
                    format_speed(network.download_speed),
                    None,
                    icon_label_gap
                )
            }),
            SystemIndicator::UploadSpeed => data.network.as_ref().map(|network| {
                indicator_info_element::<u32>(
                    icons,
                    Icons::UploadSpeed,
                    format_speed(network.upload_speed),
                    None,
                    icon_label_gap
                )
            })
        };

        element.map(|element| element.map(M::from))
    }

    /// Build the indicator widgets representing the configured subset of
    /// metrics.
    ///
    /// The gaps inside every indicator come from the themed font size carried
    /// by `appearance`, so the bar row keeps its proportions across themes.
    pub fn indicator_elements<M>(
        data: SystemInfoData,
        config: &SystemModuleConfig,
        memory_format: MemoryFormat,
        appearance: &Appearance,
        icons: &IconTheme
    ) -> Vec<Element<'static, M>>
    where
        M: 'static + From<Message>
    {
        indicators::resolve(config, &data)
            .iter()
            .filter_map(|indicator| {
                single_indicator(indicator, &data, config, memory_format, appearance, icons)
            })
            .collect()
    }

    /// Construct the condensed indicator row shown in the module section.
    /// A module declaring alternative readouts cycles them on the left button
    /// and moves the menu to the right button, the way waybar binds
    /// `format-alt`.
    pub fn build_indicator_view<M>(
        data: &SystemInfoData,
        config: &SystemModuleConfig,
        memory_format: MemoryFormat,
        appearance: &Appearance,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        let indicators =
            indicator_elements(data.clone(), config, memory_format, appearance, icons);

        let on_press = if config.has_alternatives() {
            OnModulePress::Action(Box::new(M::from(Message::NextFormat)))
        } else {
            OnModulePress::ToggleMenu(MenuType::SystemInfo)
        };

        Some((
            Row::with_children(indicators)
                .align_y(Alignment::Center)
                .spacing(scale::item_gap())
                .into(),
            Some(on_press)
        ))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn data_fixture() -> SystemInfoData {
            SystemInfoData {
                cpu_usage:         25,
                cpu_count:         8,
                memory_usage:      50,
                memory_used:       8 * 1024 * 1024 * 1024,
                memory_total:      16 * 1024 * 1024 * 1024,
                memory_swap_usage: 10,
                memory_swap_used:  1024 * 1024 * 1024,
                memory_swap_total: 10 * 1024 * 1024 * 1024,
                cpu_temperature:   Some(42),
                gpu:               None,
                disks:             vec![crate::modules::system_info::DiskData {
                    mount:         "/".to_string(),
                    used:          60 * 1024 * 1024 * 1024,
                    total:         100 * 1024 * 1024 * 1024,
                    usage_percent: 60
                }],
                network:           None
            }
        }

        #[test]
        fn indicator_row_contains_configured_entries() {
            let config = SystemModuleConfig {
                indicators: vec![SystemIndicator::Cpu, SystemIndicator::Memory],
                ..SystemModuleConfig::default()
            };

            let indicators: Vec<Element<'_, Message>> = indicator_elements(
                data_fixture(),
                &config,
                MemoryFormat::Percentage,
                &Appearance::default(),
                &IconTheme::default()
            );
            assert_eq!(indicators.len(), 2);
        }

        #[test]
        fn indicator_elements_include_network_entries_when_available() {
            let mut data = data_fixture();
            data.network = Some(crate::modules::system_info::NetworkData::new(
                "127.0.0.1".to_string(),
                2048,
                1024,
                std::time::Instant::now()
            ));

            let config = SystemModuleConfig {
                indicators: vec![SystemIndicator::IpAddress, SystemIndicator::DownloadSpeed],
                ..SystemModuleConfig::default()
            };

            let indicators: Vec<Element<'_, Message>> = indicator_elements(
                data,
                &config,
                MemoryFormat::Percentage,
                &Appearance::default(),
                &IconTheme::default()
            );
            assert_eq!(indicators.len(), 2);
        }

        #[test]
        fn format_speed_converts_large_values_to_megabytes() {
            assert_eq!(format_speed(2048), "2.0 MB/s");
        }

        #[test]
        fn format_speed_keeps_the_fraction_a_truncation_used_to_drop() {
            assert_eq!(format_speed(1999), "2.0 MB/s");
            assert_eq!(format_speed(1500), "1.5 MB/s");
            assert_eq!(format_speed(999), "999 KB/s");
        }

        #[test]
        fn the_memory_readout_follows_the_active_format() {
            let data = data_fixture();

            assert_eq!(
                memory_label(
                    MemoryFormat::Percentage,
                    None,
                    data.memory_usage,
                    data.memory_used
                ),
                "50%"
            );
            assert_eq!(
                memory_label(
                    MemoryFormat::Bytes,
                    None,
                    data.memory_usage,
                    data.memory_used
                ),
                "8.0GiB"
            );
        }

        #[test]
        fn a_prefixed_memory_readout_keeps_its_prefix_in_both_formats() {
            assert_eq!(
                memory_label(
                    MemoryFormat::Percentage,
                    Some("swap"),
                    10,
                    1024 * 1024 * 1024
                ),
                "swap 10%"
            );
            assert_eq!(
                memory_label(MemoryFormat::Bytes, Some("swap"), 10, 1024 * 1024 * 1024),
                "swap 1.0GiB"
            );
        }

        #[test]
        fn gigabytes_round_to_a_single_decimal() {
            assert_eq!(gigabytes(0), "0.0");
            assert_eq!(gigabytes(1024 * 1024 * 1024 * 3 / 2), "1.5");
        }

        #[test]
        fn a_pool_reads_as_used_against_total() {
            assert_eq!(
                used_of_total(8 * 1024 * 1024 * 1024, 16 * 1024 * 1024 * 1024),
                "8.0 / 16.0 GiB"
            );
        }
    }
}
mod window {
    //! The system monitor window: what it says, how much room it needs, and
    //! how it is drawn.
    //!
    //! Three rooms after the calendar's pattern: [`model`] states every
    //! section and row out of one sample, [`metrics`] measures the room that
    //! statement needs from the same constants the drawing uses, and
    //! [`render`] draws it. The window is measured rather than stock-sized,
    //! so the box hugs the readouts this machine actually reports.

    pub(super) mod model {
        //! Every readout of the window, stated as data before it is drawn.
        //!
        //! The statement is pure: the height measurement and the drawing both
        //! walk this list, so the two cannot disagree about what is shown.

        use hydebar_proto::config::SystemModuleConfig;

        use super::super::{
            data::SystemInfoData,
            indicators,
            view::{format_speed, gpu_title, used_of_total}
        };
        use crate::components::icons::Icons;

        /// Fill of a usage meter, judged against the shares people read as
        /// trouble.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum MeterLevel {
            Calm,
            Busy,
            Critical
        }

        /// Level a meter filled to `percent` draws in.
        ///
        /// The buckets are fixed rather than configurable: the window is a
        /// diagnostic surface, and four fifths full is where a pool starts
        /// being worth a look whatever thresholds the bar indicators carry.
        #[must_use]
        pub fn meter_level(percent: u32) -> MeterLevel {
            if percent >= 95 {
                MeterLevel::Critical
            } else if percent >= 80 {
                MeterLevel::Busy
            } else {
                MeterLevel::Calm
            }
        }

        /// Share of `total` behind `used`, in percent.
        #[must_use]
        pub fn share(used: u64, total: u64) -> u32 {
            if total == 0 {
                return 0;
            }

            ((used as f64 / total as f64) * 100.0) as u32
        }

        /// One line of the window.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum Row {
            /// A named value.
            Fact { label: String, value: String },
            /// A named value with a usage meter drawn under it.
            Meter {
                label:   String,
                value:   String,
                percent: u32
            }
        }

        /// A titled group of rows.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Section {
            pub icon:  Icons,
            pub title: &'static str,
            /// Line naming the exact source of the readings, when one
            /// matters.
            pub note:  Option<String>,
            pub rows:  Vec<Row>
        }

        fn fact(label: &str, value: String) -> Row {
            Row::Fact {
                label: label.to_owned(),
                value
            }
        }

        fn meter(label: &str, value: String, percent: u32) -> Row {
            Row::Meter {
                label: label.to_owned(),
                value,
                percent
            }
        }

        /// A pool spelled out with the share the meter next to it draws.
        fn pool(used: u64, total: u64, percent: u32) -> String {
            format!("{} ({percent}%)", used_of_total(used, total))
        }

        /// Everything the window says about the machine, in drawing order.
        ///
        /// A section the machine cannot fill is left out whole rather than
        /// drawn empty; what is missing and why is stated by
        /// [`footnotes`] instead.
        #[must_use]
        pub fn sections(data: &SystemInfoData) -> Vec<Section> {
            let mut sections = Vec::new();

            let mut processor = vec![meter(
                "Load",
                format!("{}%", data.cpu_usage),
                data.cpu_usage
            )];

            if data.cpu_count > 0 {
                processor.push(fact("Threads", data.cpu_count.to_string()));
            }

            if let Some(temperature) = data.cpu_temperature {
                processor.push(fact("Temperature", format!("{temperature}°C")));
            }

            sections.push(Section {
                icon:  Icons::Cpu,
                title: "Processor",
                note:  None,
                rows:  processor
            });

            let mut memory = vec![meter(
                "In use",
                pool(data.memory_used, data.memory_total, data.memory_usage),
                data.memory_usage
            )];

            if data.memory_total > 0 {
                memory.push(fact(
                    "Available",
                    format!(
                        "{} GiB",
                        super::super::view::gigabytes(
                            data.memory_total.saturating_sub(data.memory_used)
                        )
                    )
                ));
            }

            if data.memory_swap_total > 0 {
                memory.push(meter(
                    "Swap",
                    pool(
                        data.memory_swap_used,
                        data.memory_swap_total,
                        data.memory_swap_usage
                    ),
                    data.memory_swap_usage
                ));
            }

            sections.push(Section {
                icon:  Icons::Mem,
                title: "Memory",
                note:  None,
                rows:  memory
            });

            if let Some(gpu) = data.gpu.as_ref() {
                let mut rows = Vec::new();

                if let Some(temperature) = gpu.temperature {
                    rows.push(fact("Temperature", format!("{temperature}°C")));
                }

                if let Some(usage) = gpu.utilisation {
                    rows.push(meter("Load", format!("{usage}%"), usage));
                }

                if let Some((used, total)) = gpu.memory_used.zip(gpu.memory_total)
                    && total > 0
                {
                    let percent = share(used, total);
                    rows.push(meter("Memory", pool(used, total, percent), percent));
                }

                if !rows.is_empty() {
                    sections.push(Section {
                        icon: Icons::Gpu,
                        title: "Graphics",
                        note: Some(gpu_title(gpu)),
                        rows
                    });
                }
            }

            if !data.disks.is_empty() {
                sections.push(Section {
                    icon:  Icons::Drive,
                    title: "Storage",
                    note:  None,
                    rows:  data
                        .disks
                        .iter()
                        .map(|disk| {
                            meter(
                                &disk.mount,
                                pool(disk.used, disk.total, disk.usage_percent),
                                disk.usage_percent
                            )
                        })
                        .collect()
                });
            }

            if let Some(network) = data.network.as_ref() {
                sections.push(Section {
                    icon:  Icons::Ethernet,
                    title: "Network",
                    note:  None,
                    rows:  vec![
                        fact("Address", network.ip.clone()),
                        fact("Download", format_speed(network.download_speed)),
                        fact("Upload", format_speed(network.upload_speed)),
                    ]
                });
            }

            sections
        }

        /// Readouts this machine cannot report, each named with its reason.
        #[must_use]
        pub fn footnotes(data: &SystemInfoData, config: &SystemModuleConfig) -> Vec<String> {
            indicators::statuses(config, data)
                .into_iter()
                .filter_map(|status| {
                    let reason = status.unavailable?.reason();

                    Some(format!(
                        "{} — {reason}",
                        indicators::title(&status.indicator)
                    ))
                })
                .collect()
        }
    }

    pub(super) mod metrics {
        //! Every size of the window, and the room the whole of it takes.
        //!
        //! The box is sized from these same constants the drawing uses, so a
        //! size changed here moves the drawing and the measurement together.

        use hydebar_proto::config::SystemModuleConfig;

        use super::{
            super::data::SystemInfoData,
            model::{self, Row, Section}
        };
        use crate::components::scale;

        /// Window title size, in pixels of the reference theme.
        pub(super) const TITLE_SIZE: f32 = 18.0;

        /// Section title size, in pixels of the reference theme.
        pub(super) const SECTION_TITLE_SIZE: f32 = 12.0;

        /// Reading label and value size, in pixels of the reference theme.
        pub(super) const ROW_SIZE: f32 = 13.0;

        /// Source note and footnote size, in pixels of the reference theme.
        pub(super) const NOTE_SIZE: f32 = 11.0;

        /// Height of a usage meter track, in pixels of the reference theme.
        pub(super) const METER_HEIGHT: f32 = 5.0;

        /// Gap between a reading and the meter under it.
        pub(super) const METER_GAP: f32 = 4.0;

        /// Gap between two rows of one section.
        pub(super) const ROW_GAP: f32 = 6.0;

        /// Gap between the title, the sections and the footnotes.
        pub(super) const SECTION_GAP: f32 = 10.0;

        /// Gap between the footnote lines.
        pub(super) const FOOT_GAP: f32 = 4.0;

        /// Padding of the whole column, in pixels of the reference theme.
        pub(super) const OUTER_PADDING: f32 = 4.0;

        /// Width of the content column, in pixels of the reference theme.
        pub(super) const WIDTH: f32 = 300.0;

        /// Height one line of text at `size` occupies at the stock line
        /// height.
        fn line(size: f32) -> f32 {
            scale::scaled(size) * 1.3
        }

        /// Width of the content column alone.
        pub(super) fn column_width() -> f32 {
            scale::scaled(WIDTH)
        }

        /// Width the menu box needs, box padding included.
        pub fn content_width(font_size: f32) -> f32 {
            scale::scaled(WIDTH + 2.0 * OUTER_PADDING)
                + 2.0 * crate::menu::MENU_PADDING_EM * font_size
        }

        fn row_height(row: &Row) -> f32 {
            match row {
                Row::Fact {
                    ..
                } => line(ROW_SIZE),
                Row::Meter {
                    ..
                } => line(ROW_SIZE) + scale::scaled(METER_GAP + METER_HEIGHT)
            }
        }

        fn section_height(section: &Section) -> f32 {
            let title = line(SECTION_TITLE_SIZE);
            let note = section.note.as_ref().map_or(0.0, |_| line(NOTE_SIZE));
            let rows: f32 = section.rows.iter().map(row_height).sum();
            let inner_gaps = section.rows.len() + usize::from(section.note.is_some());
            let gaps = scale::scaled(ROW_GAP) * inner_gaps as f32;

            title + note + rows + gaps
        }

        fn footnotes_height(footnotes: &[String]) -> f32 {
            if footnotes.is_empty() {
                return 0.0;
            }

            let rule = 1.0;
            let heading = line(SECTION_TITLE_SIZE);
            let lines = footnotes.len() as f32 * line(NOTE_SIZE);
            let gaps = scale::scaled(FOOT_GAP) * (footnotes.len() as f32 + 1.0);

            rule + heading + lines + gaps
        }

        /// Height the menu content needs for the readouts it currently
        /// shows.
        pub fn content_height(data: &SystemInfoData, config: &SystemModuleConfig) -> f32 {
            let sections = model::sections(data);
            let footnotes = model::footnotes(data, config);

            let title = line(TITLE_SIZE);
            let rule = 1.0;
            let body: f32 = sections.iter().map(section_height).sum();
            let blocks = 2 + sections.len() + usize::from(!footnotes.is_empty());
            let gaps = scale::scaled(SECTION_GAP) * (blocks as f32 - 1.0);
            let padding = scale::scaled(2.0 * OUTER_PADDING);

            title + rule + body + footnotes_height(&footnotes) + gaps + padding
        }
    }

    mod render {
        //! Drawing of the window: sections, aligned value columns and themed
        //! usage meters.

        use iced::{
            Alignment, Border, Color, Element, Length, Theme,
            widget::{Column, Row as BarRow, Space, column, container, row, rule}
        };

        use super::{
            super::{Message, data::SystemInfoData},
            metrics::{
                self, FOOT_GAP, METER_GAP, METER_HEIGHT, NOTE_SIZE, OUTER_PADDING, ROW_GAP,
                ROW_SIZE, SECTION_GAP, SECTION_TITLE_SIZE, TITLE_SIZE
            },
            model::{self, MeterLevel, Row, Section}
        };
        use crate::{
            components::{
                icons::{IconTheme, icon},
                scale,
                text::text
            },
            config::SystemModuleConfig
        };

        /// Render the module menu displaying detailed system metrics.
        pub fn build_menu_view<'a>(
            data: &'a SystemInfoData,
            config: &SystemModuleConfig,
            icons: &IconTheme
        ) -> Element<'a, Message> {
            let sections = model::sections(data);
            let footnotes = model::footnotes(data, config);

            let mut content = Column::new()
                .push(text("System monitor").size(scale::scaled(TITLE_SIZE)))
                .push(rule::horizontal(1));

            for section in sections {
                content = content.push(section_view(section, icons));
            }

            if !footnotes.is_empty() {
                content = content.push(footnotes_view(footnotes));
            }

            content
                .spacing(scale::scaled(SECTION_GAP))
                .padding(scale::scaled(OUTER_PADDING))
                .width(Length::Fixed(metrics::column_width()))
                .into()
        }

        fn section_view<'a>(section: Section, icons: &IconTheme) -> Element<'a, Message> {
            let header = container(
                row![
                    icon(icons, section.icon).size(scale::scaled(SECTION_TITLE_SIZE)),
                    text(section.title).size(scale::scaled(SECTION_TITLE_SIZE))
                ]
                .align_y(Alignment::Center)
                .spacing(scale::icon_gap())
            )
            .style(secondary_text);

            let mut body = Column::new().push(header);

            if let Some(note) = section.note {
                body = body.push(
                    container(text(note).size(scale::scaled(NOTE_SIZE))).style(secondary_text)
                );
            }

            for entry in section.rows {
                body = body.push(row_view(entry));
            }

            body.spacing(scale::scaled(ROW_GAP)).into()
        }

        fn row_view<'a>(entry: Row) -> Element<'a, Message> {
            match entry {
                Row::Fact {
                    label,
                    value
                } => fact_view(label, value),
                Row::Meter {
                    label,
                    value,
                    percent
                } => column![fact_view(label, value), meter_view(percent)]
                    .spacing(scale::scaled(METER_GAP))
                    .into()
            }
        }

        /// A label on the left, its value on the right edge of the column.
        fn fact_view<'a>(label: String, value: String) -> Element<'a, Message> {
            row![
                container(text(label).size(scale::scaled(ROW_SIZE)))
                    .style(secondary_text)
                    .width(Length::Fill),
                text(value).size(scale::scaled(ROW_SIZE))
            ]
            .align_y(Alignment::Center)
            .into()
        }

        /// A usage meter: the filled share over a weak track, coloured by how
        /// full the pool is.
        fn meter_view<'a>(percent: u32) -> Element<'a, Message> {
            let percent = percent.min(100);
            let radius = scale::scaled(METER_HEIGHT / 2.0);

            let mut bar = BarRow::new();

            if percent > 0 {
                bar = bar.push(
                    container(Space::new().width(Length::Fill).height(Length::Fill))
                        .width(Length::FillPortion(percent as u16))
                        .height(Length::Fill)
                        .style(move |theme: &Theme| fill_style(theme, percent, radius))
                );
            }

            if percent < 100 {
                bar = bar.push(Space::new().width(Length::FillPortion((100 - percent) as u16)));
            }

            container(bar)
                .width(Length::Fill)
                .height(Length::Fixed(scale::scaled(METER_HEIGHT)))
                .style(move |theme: &Theme| track_style(theme, radius))
                .into()
        }

        fn footnotes_view<'a>(footnotes: Vec<String>) -> Element<'a, Message> {
            let mut body = Column::new().push(rule::horizontal(1)).push(
                container(
                    text("Not reported on this machine").size(scale::scaled(SECTION_TITLE_SIZE))
                )
                .style(secondary_text)
            );

            for entry in footnotes {
                body = body.push(
                    container(text(entry).size(scale::scaled(NOTE_SIZE))).style(secondary_text)
                );
            }

            body.spacing(scale::scaled(FOOT_GAP)).into()
        }

        fn fill_style(theme: &Theme, percent: u32, radius: f32) -> container::Style {
            let colour = match model::meter_level(percent) {
                MeterLevel::Calm => theme.palette().primary,
                MeterLevel::Busy => theme.palette().warning,
                MeterLevel::Critical => theme.palette().danger
            };

            rounded(colour.into(), radius)
        }

        fn track_style(theme: &Theme, radius: f32) -> container::Style {
            rounded(
                theme.extended_palette().background.weak.color.into(),
                radius
            )
        }

        fn rounded(background: iced::Background, radius: f32) -> container::Style {
            container::Style {
                background: Some(background),
                border: Border {
                    width:  0.0,
                    radius: radius.into(),
                    color:  Color::TRANSPARENT
                },
                ..container::Style::default()
            }
        }

        /// Secondary text in the muted shade the theme provides.
        fn secondary_text(theme: &Theme) -> container::Style {
            container::Style {
                text_color: Some(theme.extended_palette().background.weak.text),
                ..container::Style::default()
            }
        }
    }

    pub use metrics::{content_height, content_width};
    pub use render::build_menu_view;

    #[cfg(test)]
    mod tests {
        use hydebar_proto::config::{SystemIndicator, SystemModuleConfig};

        use super::{
            metrics,
            model::{self, MeterLevel, Row}
        };
        use crate::modules::system_info::{
            DiskData, SystemInfoData,
            sensors::{GpuPlacement, GpuReadings, GpuVendor}
        };

        const GIB: u64 = 1024 * 1024 * 1024;

        fn machine() -> SystemInfoData {
            SystemInfoData {
                cpu_usage:         34,
                cpu_count:         32,
                memory_usage:      15,
                memory_used:       9 * GIB,
                memory_total:      62 * GIB,
                memory_swap_usage: 6,
                memory_swap_used:  GIB / 2,
                memory_swap_total: 8 * GIB,
                cpu_temperature:   Some(56),
                gpu:               Some(GpuReadings {
                    name:         "amdgpu".to_owned(),
                    source:       Some("amdgpu junction".to_owned()),
                    vendor:       GpuVendor::Amd,
                    placement:    GpuPlacement::Discrete,
                    temperature:  Some(47),
                    utilisation:  Some(11),
                    memory_used:  Some(GIB),
                    memory_total: Some(8 * GIB)
                }),
                disks:             vec![DiskData {
                    mount:         "/".to_owned(),
                    used:          213 * GIB,
                    total:         476 * GIB,
                    usage_percent: 44
                }],
                network:           Some(crate::modules::system_info::NetworkData::new(
                    "192.168.1.5".to_owned(),
                    1500,
                    120,
                    std::time::Instant::now()
                ))
            }
        }

        fn titles(data: &SystemInfoData) -> Vec<&'static str> {
            model::sections(data)
                .into_iter()
                .map(|section| section.title)
                .collect()
        }

        #[test]
        fn meters_change_colour_at_the_stated_shares() {
            assert_eq!(model::meter_level(0), MeterLevel::Calm);
            assert_eq!(model::meter_level(79), MeterLevel::Calm);
            assert_eq!(model::meter_level(80), MeterLevel::Busy);
            assert_eq!(model::meter_level(94), MeterLevel::Busy);
            assert_eq!(model::meter_level(95), MeterLevel::Critical);
            assert_eq!(model::meter_level(100), MeterLevel::Critical);
        }

        #[test]
        fn a_share_is_a_percentage_and_an_empty_pool_has_none() {
            assert_eq!(model::share(GIB, 8 * GIB), 12);
            assert_eq!(model::share(0, 8 * GIB), 0);
            assert_eq!(model::share(GIB, 0), 0);
        }

        #[test]
        fn a_full_machine_states_every_section_in_order() {
            assert_eq!(
                titles(&machine()),
                vec!["Processor", "Memory", "Graphics", "Storage", "Network"]
            );
        }

        #[test]
        fn a_machine_without_swap_shows_no_swap_row() {
            let mut data = machine();
            data.memory_swap_total = 0;

            let sections = model::sections(&data);
            let memory = sections
                .iter()
                .find(|section| section.title == "Memory")
                .expect("memory section");

            assert_eq!(memory.rows.len(), 2);
            assert!(matches!(
                &memory.rows[1],
                Row::Fact { label, value } if label == "Available" && value == "53.0 GiB"
            ));
        }

        #[test]
        fn swap_reads_as_a_pool_when_the_machine_has_one() {
            let sections = model::sections(&machine());
            let memory = sections
                .iter()
                .find(|section| section.title == "Memory")
                .expect("memory section");

            assert!(matches!(
                &memory.rows[2],
                Row::Meter { label, value, percent: 6 }
                    if label == "Swap" && value == "0.5 / 8.0 GiB (6%)"
            ));
        }

        /// The window says how many threads the load percentage averages
        /// over, so `3%` on a 32-thread machine reads as the light load it
        /// is.
        #[test]
        fn the_processor_section_counts_its_threads() {
            let sections = model::sections(&machine());
            let processor = sections
                .iter()
                .find(|section| section.title == "Processor")
                .expect("processor section");

            assert!(matches!(
                &processor.rows[1],
                Row::Fact { label, value } if label == "Threads" && value == "32"
            ));
        }

        #[test]
        fn the_graphics_section_names_its_source() {
            let sections = model::sections(&machine());
            let graphics = sections
                .iter()
                .find(|section| section.title == "Graphics")
                .expect("graphics section");

            assert_eq!(graphics.note.as_deref(), Some("Graphics (amdgpu junction)"));
            assert_eq!(graphics.rows.len(), 3);
        }

        #[test]
        fn transfer_rates_read_in_the_window_as_on_the_bar() {
            let sections = model::sections(&machine());
            let network = sections
                .iter()
                .find(|section| section.title == "Network")
                .expect("network section");

            assert!(matches!(
                &network.rows[1],
                Row::Fact { label, value } if label == "Download" && value == "1.5 MB/s"
            ));
        }

        #[test]
        fn a_machine_reporting_nothing_still_states_load_and_memory() {
            assert_eq!(
                titles(&SystemInfoData::default()),
                vec!["Processor", "Memory"]
            );
        }

        #[test]
        fn footnotes_name_what_the_machine_cannot_report() {
            let footnotes =
                model::footnotes(&SystemInfoData::default(), &SystemModuleConfig::default());

            assert!(
                footnotes
                    .iter()
                    .any(|line| line.starts_with("CPU temperature"))
            );
            assert!(footnotes.iter().any(|line| line.starts_with("Swap usage")));
        }

        #[test]
        fn a_listed_disk_that_is_not_mounted_is_a_footnote() {
            let config = SystemModuleConfig {
                indicators: vec![SystemIndicator::Disk("/data".to_owned())],
                ..SystemModuleConfig::default()
            };

            let footnotes = model::footnotes(&machine(), &config);

            assert!(footnotes.iter().any(|line| line.starts_with("Disk /data")));
        }

        #[test]
        fn the_window_grows_with_what_it_shows() {
            let config = SystemModuleConfig::default();
            let bare = metrics::content_height(&SystemInfoData::default(), &config);
            let full = metrics::content_height(&machine(), &config);

            assert!(full > bare);
            assert!(bare > 0.0);
            assert!(metrics::content_width(10.0) > 0.0);
        }
    }
}

pub use data::{DiskData, NetworkData, SystemInfoData, SystemInfoSampler};
use hydebar_proto::config::{Appearance, MemoryFormat, SystemModuleConfig};
use iced::Element;
pub use indicators::{IndicatorStatus, Unavailable};
pub use runtime::REFRESH_INTERVAL;
pub use sensors::{GpuPlacement, GpuReadings, GpuVendor, HardwareSensors};
pub use view::{build_indicator_view, indicator_elements, single_indicator};
pub(crate) use view::{gigabytes, used_of_total};
pub use window::build_menu_view;

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, attention::PollSchedule, components::icons::IconTheme, event_bus::ModuleEvent,
    format_cycle::FormatCycle
};

/// Messages published by the system information module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Readouts that differ from the ones currently on screen.
    Sampled(SystemInfoData),
    /// Switch to the next configured readout, wrapping after the last one.
    NextFormat
}

/// Module responsible for sampling and presenting local system metrics.
pub struct SystemInfo {
    data:    SystemInfoData,
    polling: runtime::PollingTask,
    format:  FormatCycle
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            data:    SystemInfoSampler::new().sample_with_extras(),
            polling: runtime::PollingTask::new(),
            format:  FormatCycle::new()
        }
    }
}

impl SystemInfo {
    /// React to module messages by updating cached metrics when necessary.
    pub fn update(&mut self, message: Message, config: &SystemModuleConfig) {
        match message {
            Message::Sampled(data) => {
                self.data = data;
            }
            Message::NextFormat => {
                self.format.advance(&config.memory.format_alt);
            }
        }
    }

    /// Memory readout the active index selects.
    pub fn active_memory_format(&self, config: &SystemModuleConfig) -> MemoryFormat {
        *self
            .format
            .resolve(&config.memory.format, &config.memory.format_alt)
    }

    /// Latest sample, for the thin bar entries and the hover hints that
    /// render from it.
    pub fn data(&self) -> &SystemInfoData {
        &self.data
    }

    /// Width the monitor window asks the screen for.
    ///
    /// Stated by the module so the box hugs its value columns; a stock menu
    /// width left a blank margin beside readouts that cannot grow into it.
    pub fn content_width(font_size: f32) -> f32 {
        window::content_width(font_size)
    }

    /// Height the window needs for the readouts this machine reports.
    pub fn content_height(&self, config: &SystemModuleConfig) -> f32 {
        window::content_height(&self.data, config)
    }

    /// Render the menu entry exposing detailed system information.
    pub fn menu_view(
        &self,
        config: &SystemModuleConfig,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        window::build_menu_view(&self.data, config, icons)
    }
}

impl<M> Module<M> for SystemInfo
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = (&'a SystemModuleConfig, &'a Appearance, &'a IconTheme);
    type RegistrationData<'a> = &'a SystemModuleConfig;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        let sender = ctx.module_sender(ModuleEvent::SystemInfo);
        self.polling
            .spawn(ctx, sender, config.gpu.device.as_deref());

        Ok(())
    }

    /// Stops sampling the machine once the readout leaves the bar.
    ///
    /// Reading every CPU, disk and interface costs real work, so a layout that
    /// dropped the module must not keep paying for it every few seconds.
    fn deregister(&mut self) {
        self.polling.abort();
    }

    /// Refreshes faster only while somebody is looking.
    ///
    /// The module's own task keeps the bar current on the resting cadence;
    /// the attended cadence exists for the window and the hover hints, whose
    /// readouts would otherwise be up to a whole interval old the moment
    /// they open.
    fn poll_schedule(&self) -> Option<PollSchedule> {
        Some(PollSchedule::only_when_attended(runtime::ATTENDED_INTERVAL))
    }

    fn poll(&mut self, _: &ModuleContext) -> Result<(), ModuleError> {
        self.polling.poke();

        Ok(())
    }

    fn view(
        &self,
        (config, appearance, icons): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        view::build_indicator_view(
            &self.data,
            config,
            self.active_memory_format(config),
            appearance,
            icons
        )
    }
}

#[cfg(test)]
mod tests {
    use hydebar_proto::config::SystemInfoMemory;

    use super::*;

    fn config(format: MemoryFormat, alternatives: &[MemoryFormat]) -> SystemModuleConfig {
        SystemModuleConfig {
            memory: SystemInfoMemory {
                format,
                format_alt: alternatives.to_vec(),
                ..SystemInfoMemory::default()
            },
            ..SystemModuleConfig::default()
        }
    }

    #[test]
    fn a_press_walks_the_configured_readouts_and_wraps_around() {
        let config = config(MemoryFormat::Bytes, &[MemoryFormat::Percentage]);
        let mut module = SystemInfo::default();

        assert_eq!(module.active_memory_format(&config), MemoryFormat::Bytes);

        module.update(Message::NextFormat, &config);
        assert_eq!(
            module.active_memory_format(&config),
            MemoryFormat::Percentage
        );

        module.update(Message::NextFormat, &config);
        assert_eq!(module.active_memory_format(&config), MemoryFormat::Bytes);
    }

    #[test]
    fn a_module_without_alternatives_keeps_its_readout() {
        let config = config(MemoryFormat::Percentage, &[]);
        let mut module = SystemInfo::default();

        module.update(Message::NextFormat, &config);

        assert_eq!(
            module.active_memory_format(&config),
            MemoryFormat::Percentage
        );
    }
}
