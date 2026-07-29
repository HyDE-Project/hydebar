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
    /// Processor temperature, absent on a machine that reports none.
    pub cpu_temperature:   Option<i32>,
    /// Graphics readings, absent when the machine exposes no graphics device.
    pub gpu:               Option<GpuReadings>,
    pub disks:             Vec<(String, u32)>,
    pub network:           Option<NetworkData>
}

impl Default for SystemInfoData {
    fn default() -> Self {
        Self {
            cpu_usage:         0,
            memory_usage:      0,
            memory_used:       0,
            memory_swap_usage: 0,
            memory_swap_used:  0,
            cpu_temperature:   None,
            gpu:               None,
            disks:             Vec::new(),
            network:           None
        }
    }
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
/// Temperatures come from [`HardwareSensors`] rather than from `sysinfo`: the
/// panel has to know which chip a reading belongs to, has to pair it with the
/// load the graphics driver publishes elsewhere, and has to read only the two
/// files it settled on instead of the whole subsystem on every tick.
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
        let (memory_usage, memory_used) =
            memory_share(self.system.total_memory(), self.system.available_memory());
        let (memory_swap_usage, memory_swap_used) =
            memory_share(self.system.total_swap(), self.system.free_swap());

        SystemInfoData {
            cpu_usage,
            memory_usage,
            memory_used,
            memory_swap_usage,
            memory_swap_used,
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
        let (memory_usage, memory_used) =
            memory_share(self.system.total_memory(), self.system.available_memory());
        let (memory_swap_usage, memory_swap_used) =
            memory_share(self.system.total_swap(), self.system.free_swap());

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
/// kernel currently holds free: for RAM that is `MemAvailable`, which counts
/// the reclaimable page cache as unused. Subtracting `MemFree` instead would
/// bill every cached page to the user and report a machine with a warm cache
/// as nearly full.
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
