//! The everyday sampling: processor, memory and the sensor readings.

use sysinfo::{Disks, Networks};

use super::{
    SystemInfoData, SystemInfoSampler,
    metrics::{cpu_counters, memory_share, stamp_environment}
};
use crate::modules::system_info::sensors::{HardwareSensors, Readings};

impl SystemInfoSampler {
    /// Instantiate a sampler with refreshed sysinfo collections.
    #[must_use]
    pub fn new() -> Self {
        Self {
            system:       sysinfo::System::new_with_specifics(
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
    pub const fn only_cpu_and_memory(&mut self) {
        self.full = false;
    }

    pub(super) fn ensure_disks(&mut self) {
        if self.disks.is_none() {
            self.disks = Some(Disks::new_with_refreshed_list());
        }
    }

    pub(super) fn ensure_networks(&mut self) {
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

        let (cpu_usage, cpu_count) = cpu_counters(&self.system);
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
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn sampler_produces_data() {
        let mut sampler = SystemInfoSampler::new();
        let data = sampler.sample();

        assert!(data.cpu_usage <= 100);
        assert!(data.memory_usage <= 100);
        assert!(data.memory_swap_usage <= 100);
    }
}
