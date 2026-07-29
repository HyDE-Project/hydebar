use std::time::Instant;

use itertools::Itertools;
use sysinfo::{Components, Disks, Networks, System};

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
    pub fn new(ip: String, download_speed: u32, upload_speed: u32, last_check: Instant) -> Self {
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

/// Aggregated system information consumed by the UI layer.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemInfoData {
    pub cpu_usage:         u32,
    pub memory_usage:      u32,
    /// Memory in use, in bytes, behind [`Self::memory_usage`].
    pub memory_used:       u64,
    pub memory_swap_usage: u32,
    /// Swap in use, in bytes, behind [`Self::memory_swap_usage`].
    pub memory_swap_used:  u64,
    pub temperature:       Option<i32>,
    pub disks:             Vec<(String, u32)>,
    pub network:           Option<NetworkData>
}

impl SystemInfoData {
    /// Reports whether two samples would paint the same readouts.
    ///
    /// [`NetworkData::last_check`] records when a sample was taken rather than
    /// anything the bar draws, and it moves on every tick, so plain equality
    /// would never hold. Excluding it lets an idle machine skip the repaint the
    /// sample would otherwise force.
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
            && self.memory_usage == other.memory_usage
            && self.memory_used == other.memory_used
            && self.memory_swap_usage == other.memory_swap_usage
            && self.memory_swap_used == other.memory_swap_used
            && self.temperature == other.temperature
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
#[derive(Debug)]
pub struct SystemInfoSampler {
    system:       System,
    components:   Option<Components>,
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
            components:   None,
            disks:        None,
            networks:     None,
            last_network: None
        }
    }

    fn ensure_components(&mut self) {
        if self.components.is_none() {
            self.components = Some(Components::new_with_refreshed_list());
        }
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
        let (memory_usage, memory_used) =
            memory_share(self.system.total_memory(), self.system.available_memory());
        let (memory_swap_usage, memory_swap_used) =
            memory_share(self.system.total_swap(), self.system.free_swap());

        let temperature = None;

        let disks = Vec::new();

        let network = None;

        SystemInfoData {
            cpu_usage,
            memory_usage,
            memory_used,
            memory_swap_usage,
            memory_swap_used,
            temperature,
            disks,
            network
        }
    }

    pub fn sample_with_extras(&mut self) -> SystemInfoData {
        self.ensure_components();
        self.ensure_disks();
        self.ensure_networks();

        self.system
            .refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing().with_cpu_usage());
        self.system.refresh_memory();

        if let Some(ref mut components) = self.components {
            components.refresh(true);
        }
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
        let (memory_usage, memory_used) =
            memory_share(self.system.total_memory(), self.system.available_memory());
        let (memory_swap_usage, memory_swap_used) =
            memory_share(self.system.total_swap(), self.system.free_swap());

        let temperature = self
            .components
            .as_ref()
            .and_then(|components| cpu_temperature(components));

        let disks = self
            .disks
            .as_ref()
            .map(|disks| {
                disks
                    .iter()
                    .filter(|disk| !disk.is_removable() && disk.total_space() != 0)
                    .map(|disk| {
                        let mount_point = disk.mount_point().to_string_lossy().to_string();
                        let usage = percentage(
                            disk.total_space().saturating_sub(disk.available_space()),
                            disk.total_space()
                        );

                        (mount_point, usage)
                    })
                    .sorted_by(|a, b| a.0.cmp(&b.0))
                    .collect()
            })
            .unwrap_or_default();

        SystemInfoData {
            cpu_usage,
            memory_usage,
            memory_used,
            memory_swap_usage,
            memory_swap_used,
            temperature,
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
/// kernel currently holds free: for RAM that is `MemAvailable`, which counts
/// the reclaimable page cache as unused. Subtracting `MemFree` instead would
/// bill every cached page to the user and report a machine with a warm cache
/// as nearly full.
fn memory_share(total: u64, unused: u64) -> (u32, u64) {
    let used = total.saturating_sub(unused);

    (percentage(used, total), used)
}

/// Chips whose readings stand for a CPU die or package temperature.
const CPU_TEMPERATURE_CHIPS: [&str; 8] = [
    "k10temp",
    "zenpower",
    "zenpower3",
    "coretemp",
    "k8temp",
    "cpu_thermal",
    "cpu-thermal",
    "soc_thermal"
];

/// Sensor names a CPU chip gives its package reading, best candidate first.
const CPU_PACKAGE_SENSORS: [&str; 5] = ["tdie", "tctl", "package id 0", "package", "cpu"];

/// Sensor names a CPU chip gives its per-core and per-die readings.
const CPU_CORE_SENSORS: [&str; 3] = ["core", "tccd", "die"];

/// Rank of a per-core reading, worse than any package reading.
const CORE_SENSOR_RANK: u8 = 100;

/// Rank of a reading whose name says nothing about what it measures.
const UNNAMED_SENSOR_RANK: u8 = 200;

/// Splits a `sysinfo` component label into the chip and the sensor it names.
///
/// The crate builds labels as `"<chip> <sensor>"`, falling back to
/// `"<chip> temp<n>"` for a sensor the driver leaves unlabelled.
fn split_chip_and_sensor(label: &str) -> (&str, &str) {
    match label.split_once(' ') {
        Some((chip, sensor)) => (strip_chip_index(chip), sensor),
        None => (strip_chip_index(label), "")
    }
}

/// Drops the `_0` style index the kernel appends to some chip names.
fn strip_chip_index(chip: &str) -> &str {
    match chip.rsplit_once('_') {
        Some((base, index))
            if !base.is_empty()
                && !index.is_empty()
                && index.chars().all(|c| c.is_ascii_digit()) =>
        {
            base
        }
        _ => chip
    }
}

/// Rank of a hwmon reading as a stand-in for the CPU temperature.
///
/// Lower sorts better, and [`None`] marks a reading that never qualifies. The
/// chip decides the first component, so a CPU chip always beats the board-wide
/// ACPI zone that reports whichever node the firmware happens to expose; the
/// sensor decides the second, so the package reading beats a single core.
fn cpu_temperature_rank(label: &str) -> Option<(u8, u8)> {
    let (chip, sensor) = split_chip_and_sensor(label);
    let chip = chip.to_ascii_lowercase();
    let sensor = sensor.to_ascii_lowercase();

    let chip_rank = if CPU_TEMPERATURE_CHIPS.contains(&chip.as_str()) {
        0
    } else if chip == "x86_pkg_temp" {
        1
    } else if chip == "acpitz" {
        2
    } else {
        return None;
    };

    let sensor_rank = CPU_PACKAGE_SENSORS
        .iter()
        .position(|candidate| sensor.starts_with(candidate))
        .map(|position| position as u8)
        .or_else(|| {
            CPU_CORE_SENSORS
                .iter()
                .any(|candidate| sensor.starts_with(candidate))
                .then_some(CORE_SENSOR_RANK)
        })
        .unwrap_or(UNNAMED_SENSOR_RANK);

    Some((chip_rank, sensor_rank))
}

/// Best CPU temperature among labelled hwmon readings, in whole degrees.
///
/// The value is truncated rather than rounded so the bar agrees with the
/// `sensors` readout every other tool prints.
fn best_cpu_temperature<'a, I>(readings: I) -> Option<i32>
where
    I: IntoIterator<Item = (&'a str, f32)>
{
    readings
        .into_iter()
        .filter_map(|(label, temperature)| Some((cpu_temperature_rank(label)?, temperature)))
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, temperature)| temperature as i32)
}

/// CPU temperature reported by the refreshed component list.
fn cpu_temperature(components: &Components) -> Option<i32> {
    best_cpu_temperature(
        components
            .iter()
            .filter_map(|component| Some((component.label(), component.temperature()?)))
    )
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
    fn the_cpu_chip_beats_the_board_thermal_zone() {
        assert_eq!(
            best_cpu_temperature([("acpitz_0 temp1", 80.0), ("k10temp Tctl", 65.6)]),
            Some(65)
        );
    }

    #[test]
    fn the_package_reading_beats_a_single_core() {
        assert_eq!(
            best_cpu_temperature([
                ("coretemp Core 0", 71.0),
                ("coretemp Package id 0", 58.0),
                ("coretemp Core 1", 69.0)
            ]),
            Some(58)
        );
    }

    #[test]
    fn the_die_reading_beats_the_control_reading() {
        assert_eq!(
            best_cpu_temperature([("k10temp Tctl", 75.0), ("k10temp Tdie", 48.0)]),
            Some(48)
        );
    }

    #[test]
    fn a_core_reading_stands_in_when_the_chip_names_no_package() {
        assert_eq!(
            best_cpu_temperature([("acpitz temp1", 80.0), ("k10temp Tccd1", 61.0)]),
            Some(61)
        );
    }

    #[test]
    fn the_thermal_zone_stands_in_when_no_cpu_chip_reports() {
        assert_eq!(
            best_cpu_temperature([
                ("amdgpu edge", 52.0),
                ("nvme Composite YMTC PC411-2TB-B", 42.8),
                ("acpitz_0 temp1", 47.0)
            ]),
            Some(47)
        );
    }

    #[test]
    fn readings_from_other_hardware_never_qualify() {
        assert_eq!(
            best_cpu_temperature([
                ("amdgpu edge", 52.0),
                ("nvme Sensor 1 YMTC PC411-2TB-B", 42.8),
                ("mt7925_phy0 temp1", 49.0),
                ("r8169_0_c100:00 temp1", 51.0)
            ]),
            None
        );
    }

    #[test]
    fn an_empty_component_list_reports_no_temperature() {
        assert_eq!(best_cpu_temperature([]), None);
    }

    #[test]
    fn a_chip_index_suffix_does_not_hide_the_chip() {
        assert_eq!(split_chip_and_sensor("acpitz_0 temp1"), ("acpitz", "temp1"));
        assert_eq!(split_chip_and_sensor("k10temp Tctl"), ("k10temp", "Tctl"));
        assert_eq!(
            split_chip_and_sensor("cpu_thermal temp1"),
            ("cpu_thermal", "temp1")
        );
        assert_eq!(split_chip_and_sensor("coretemp"), ("coretemp", ""));
    }

    #[test]
    fn the_package_sensor_outranks_the_bare_chip_name() {
        assert!(cpu_temperature_rank("k10temp Tctl") < cpu_temperature_rank("k10temp temp1"));
        assert!(cpu_temperature_rank("k10temp temp1") < cpu_temperature_rank("acpitz temp1"));
        assert_eq!(cpu_temperature_rank("nvme Composite"), None);
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
