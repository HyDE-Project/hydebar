//! Readouts of the machine as data, and the sampler that captures them.
//!
//! The types here are what the rest of the module draws from; the
//! capturing lives beside them: [`sampler`] holds the sampling itself,
//! [`network`] the speed arithmetic between two looks at the counters,
//! and [`metrics`] the shared shares and identity stamping.

use std::time::Instant;

use super::sensors::GpuReadings;

pub(super) mod hardware;
mod extras;
mod metrics;
mod network;
mod sampler;

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
    #[must_use]
    pub const fn new(
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
    #[must_use]
    pub const fn last_check(&self) -> Instant {
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SystemInfoData {
    pub cpu_usage:              u32,
    /// Logical processors the load is averaged over; zero before the
    /// first real sample.
    pub cpu_count:              u32,
    pub memory_usage:           u32,
    /// Memory in use, in bytes, behind [`Self::memory_usage`].
    pub memory_used:            u64,
    /// Memory installed, in bytes.
    pub memory_total:           u64,
    pub memory_swap_usage:      u32,
    /// Swap in use, in bytes, behind [`Self::memory_swap_usage`].
    pub memory_swap_used:       u64,
    /// Swap configured, in bytes; zero on a machine without swap.
    pub memory_swap_total:      u64,
    /// Processor temperature, absent on a machine that reports none.
    pub cpu_temperature:        Option<i32>,
    /// Sensor the processor temperature is read from.
    pub cpu_temperature_source: Option<String>,
    /// Graphics readings, absent when the machine exposes no graphics
    /// device.
    pub gpu:                    Option<GpuReadings>,
    pub disks:                  Vec<DiskData>,
    pub network:                Option<NetworkData>,
    /// Processor model name, as the kernel states it.
    pub cpu_model:              Option<String>,
    /// Physical cores behind [`Self::cpu_count`] logical threads.
    pub cpu_cores:              Option<u32>,
    /// Highest boost frequency the processor is rated for, in MHz.
    pub cpu_max_mhz:            Option<u32>,
    /// Fastest core at the moment of the sample, in MHz, damped to
    /// 50 MHz steps so an idle machine does not repaint on jitter.
    pub cpu_current_mhz:        Option<u32>,
    /// Frequency governor steering the processor.
    pub cpu_governor:           Option<String>,
    /// Microcode revision the processor runs — the freshness of the
    /// firmware side of its driver stack.
    pub cpu_microcode:          Option<String>,
    /// Kernel release, the freshness of the in-tree drivers.
    pub kernel:                 Option<String>,
    /// Page cache and reclaimable slab, in bytes: memory in use that an
    /// allocation could still claim back.
    pub memory_cached:          u64,
    /// The device swap lives on, with its compression when it is zram.
    pub swap_backend:           Option<String>
}

impl SystemInfoData {
    /// Reports whether two samples would paint the same readouts.
    ///
    /// [`NetworkData::last_check`] records when a sample was taken rather
    /// than anything the bar draws, and it moves on every tick, so
    /// plain equality would never hold. Excluding it lets an idle
    /// machine skip the repaint the sample would otherwise force.
    #[must_use]
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

/// Samples system metrics using the [`sysinfo`] crate.
///
/// Temperatures come from [`super::sensors::HardwareSensors`] rather than
/// from `sysinfo`: the panel has to know which chip a reading belongs to,
/// has to pair it with the load the graphics driver publishes elsewhere,
/// and has to read only the two files it settled on instead of the whole
/// subsystem on every tick.
#[derive(Debug)]
pub struct SystemInfoSampler {
    system:       sysinfo::System,
    sensors:      super::sensors::HardwareSensors,
    disks:        Option<sysinfo::Disks>,
    networks:     Option<sysinfo::Networks>,
    last_network: Option<network::NetworkSnapshot>,
    full:         bool
}

impl Default for SystemInfoSampler {
    fn default() -> Self {
        Self::new()
    }
}
