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

pub(super) mod hardware;

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
    /// Sensor the processor temperature is read from.
    pub cpu_temperature_source: Option<String>,
    /// Graphics readings, absent when the machine exposes no graphics
    /// device.
    pub gpu:               Option<GpuReadings>,
    pub disks:             Vec<DiskData>,
    pub network:           Option<NetworkData>,
    /// Processor model name, as the kernel states it.
    pub cpu_model:         Option<String>,
    /// Physical cores behind [`Self::cpu_count`] logical threads.
    pub cpu_cores:         Option<u32>,
    /// Highest boost frequency the processor is rated for, in MHz.
    pub cpu_max_mhz:       Option<u32>,
    /// Fastest core at the moment of the sample, in MHz, damped to
    /// 50 MHz steps so an idle machine does not repaint on jitter.
    pub cpu_current_mhz:   Option<u32>,
    /// Frequency governor steering the processor.
    pub cpu_governor:      Option<String>,
    /// Microcode revision the processor runs — the freshness of the
    /// firmware side of its driver stack.
    pub cpu_microcode:     Option<String>,
    /// Kernel release, the freshness of the in-tree drivers.
    pub kernel:            Option<String>,
    /// Page cache and reclaimable slab, in bytes: memory in use that an
    /// allocation could still claim back.
    pub memory_cached:     u64,
    /// The device swap lives on, with its compression when it is zram.
    pub swap_backend:      Option<String>
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
            && self.cpu_current_mhz == other.cpu_current_mhz
            && self.cpu_governor == other.cpu_governor
            && self.memory_cached == other.memory_cached
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
    last_network: Option<NetworkSnapshot>,
    full:         bool
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
            last_network: None,
            full:         true
        }
    }

    /// Pins the graphics device the readings come from.
    pub fn prefer_gpu(&mut self, preferred: Option<&str>) {
        self.sensors.prefer_gpu(preferred);
    }

    /// Narrows the sampling to the processor and memory readouts.
    ///
    /// A layout hosting only the processor and memory entries never draws
    /// a disk, an interface or a sensor, and a sampler that read them
    /// anyway would walk every mount and the whole hwmon tree a dozen
    /// times a minute for nobody.
    pub fn only_cpu_and_memory(&mut self) {
        self.full = false;
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

        let mut data = SystemInfoData {
            cpu_usage,
            cpu_count,
            memory_usage,
            memory_used,
            memory_total,
            memory_swap_usage,
            memory_swap_used,
            memory_swap_total,
            ..SystemInfoData::default()
        };
        stamp_environment(&mut data);

        data
    }

    /// Captures whatever the sampler's scope asks for.
    ///
    /// The full scope reads everything the monitor window can show. The
    /// narrow one — see [`Self::only_cpu_and_memory`] — keeps the
    /// processor and memory counters and the sensors, whose temperature
    /// the standalone processor window states, and skips the walk over
    /// every mount and interface nobody renders.
    pub fn sample_scoped(&mut self) -> SystemInfoData {
        if self.full {
            self.sample_with_extras()
        } else {
            let mut data = self.sample();
            let Readings {
                cpu,
                cpu_source,
                gpu
            } = self.sensors.read();
            data.cpu_temperature = cpu;
            data.cpu_temperature_source = cpu_source;
            data.gpu = gpu;

            data
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
            cpu_source: cpu_temperature_source,
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

        let mut data = SystemInfoData {
            cpu_usage,
            cpu_count,
            memory_usage,
            memory_used,
            memory_total,
            memory_swap_usage,
            memory_swap_used,
            memory_swap_total,
            cpu_temperature,
            cpu_temperature_source,
            gpu,
            disks,
            network,
            ..SystemInfoData::default()
        };
        stamp_environment(&mut data);

        data
    }
}

/// Stamps the machine's identity and its living readings onto a sample.
fn stamp_environment(data: &mut SystemInfoData) {
    let identity = hardware::identity();

    data.cpu_model = identity.model.clone();
    data.cpu_cores = identity.cores;
    data.cpu_max_mhz = identity.max_mhz;
    data.cpu_microcode = identity.microcode.clone();
    data.kernel = identity.kernel.clone();
    data.swap_backend = identity.swap.clone();
    data.cpu_current_mhz = hardware::current_mhz();
    data.cpu_governor = hardware::governor();
    data.memory_cached = hardware::cached_bytes();
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
