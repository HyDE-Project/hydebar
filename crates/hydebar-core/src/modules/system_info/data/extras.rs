//! The full sample: the everyday readouts plus every mount and
//! interface the monitor window can show.

use std::time::Instant;

use itertools::Itertools;

use super::{
    DiskData, SystemInfoData, SystemInfoSampler,
    metrics::{cpu_counters, memory_share, percentage, stamp_environment},
    network::NetworkSnapshot
};
use crate::modules::system_info::sensors::Readings;

impl SystemInfoSampler {
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

        let (cpu_usage, cpu_count) = cpu_counters(&self.system);
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
