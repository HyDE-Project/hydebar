//! The remaining sections of the monitor, and the whole window they
//! line up into.

use hydebar_proto::config::SystemModuleConfig;

use super::{
    super::super::{data::SystemInfoData, indicators, view},
    Row, Section, fact, meter, pool, share
};
use crate::components::icons::Icons;

/// Rows of the memory section, in drawing order.
fn memory_rows(data: &SystemInfoData) -> Vec<Row> {
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
                view::gigabytes(data.memory_total.saturating_sub(data.memory_used))
            )
        ));
    }

    if data.memory_cached > 0 {
        memory.push(fact(
            "Cached",
            format!("{} GiB", view::gigabytes(data.memory_cached))
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

        if let Some(backend) = data.swap_backend.as_ref() {
            memory.push(fact("Swap device", backend.clone()));
        }
    }

    memory
}

/// The memory section of the monitor.
#[must_use]
pub fn memory_section(data: &SystemInfoData) -> Section {
    Section {
        icon:  Icons::Mem,
        title: "Memory",
        note:  None,
        rows:  memory_rows(data)
    }
}

/// The graphics section, absent on a machine reporting nothing of it.
#[must_use]
pub fn graphics_section(data: &SystemInfoData) -> Option<Section> {
    let gpu = data.gpu.as_ref()?;
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

    if rows.is_empty() {
        return None;
    }

    Some(Section {
        icon: Icons::Gpu,
        title: "Graphics",
        note: Some(view::gpu_title(gpu)),
        rows
    })
}

/// The storage section, absent when no disk is reported.
#[must_use]
pub fn storage_section(data: &SystemInfoData) -> Option<Section> {
    if data.disks.is_empty() {
        return None;
    }

    Some(Section {
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
    })
}

/// The network section, absent while no interface reports.
#[must_use]
pub fn network_section(data: &SystemInfoData) -> Option<Section> {
    let network = data.network.as_ref()?;

    Some(Section {
        icon:  Icons::Ethernet,
        title: "Network",
        note:  None,
        rows:  vec![
            fact("Address", network.ip.clone()),
            fact("Download", view::format_speed(network.download_speed)),
            fact("Upload", view::format_speed(network.upload_speed)),
        ]
    })
}

/// Everything the window says about the machine, in drawing order.
///
/// A section the machine cannot fill is left out whole rather than
/// drawn empty; what is missing and why is stated by
/// [`footnotes`] instead.
#[must_use]
pub fn sections(data: &SystemInfoData) -> Vec<Section> {
    let mut sections = vec![super::processor_section_full(data), memory_section(data)];

    sections.extend(graphics_section(data));
    sections.extend(storage_section(data));
    sections.extend(network_section(data));

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::system_info::{
        DiskData,
        sensors::{GpuPlacement, GpuReadings, GpuVendor}
    };

    const GIB: u64 = 1024 * 1024 * 1024;

    fn silent_gpu() -> GpuReadings {
        GpuReadings {
            name:         "amdgpu".to_owned(),
            source:       None,
            vendor:       GpuVendor::Amd,
            placement:    GpuPlacement::Discrete,
            temperature:  None,
            utilisation:  None,
            memory_used:  None,
            memory_total: None
        }
    }

    fn row_with_label<'a>(rows: &'a [Row], wanted: &str) -> Option<&'a Row> {
        rows.iter().find(|row| match row {
            Row::Fact {
                label, ..
            }
            | Row::Meter {
                label, ..
            } => label == wanted
        })
    }

    #[test]
    fn an_empty_memory_pool_hides_the_available_row() {
        let rows = memory_section(&SystemInfoData::default()).rows;

        assert_eq!(rows.len(), 1);
        assert!(matches!(
            &rows[0],
            Row::Meter { label, .. } if label == "In use"
        ));
    }

    #[test]
    fn cached_memory_reads_in_gibibytes() {
        let data = SystemInfoData {
            memory_cached: 2 * GIB,
            ..SystemInfoData::default()
        };

        let rows = memory_section(&data).rows;

        assert!(matches!(
            row_with_label(&rows, "Cached"),
            Some(Row::Fact { value, .. }) if value == "2.0 GiB"
        ));
    }

    #[test]
    fn the_swap_device_row_needs_a_swap_pool() {
        let backend = Some("zram0 (zstd)".to_owned());
        let without_pool = SystemInfoData {
            swap_backend: backend.clone(),
            ..SystemInfoData::default()
        };
        let with_pool = SystemInfoData {
            swap_backend: backend,
            memory_swap_total: 8 * GIB,
            ..SystemInfoData::default()
        };

        assert!(row_with_label(&memory_section(&without_pool).rows, "Swap device").is_none());
        assert!(matches!(
            row_with_label(&memory_section(&with_pool).rows, "Swap device"),
            Some(Row::Fact { value, .. }) if value == "zram0 (zstd)"
        ));
    }

    #[test]
    fn a_gpu_reporting_nothing_leaves_the_graphics_window_out() {
        let data = SystemInfoData {
            gpu: Some(silent_gpu()),
            ..SystemInfoData::default()
        };

        assert!(graphics_section(&data).is_none());
    }

    #[test]
    fn gpu_memory_needs_a_nonzero_total_to_draw_a_meter() {
        let data = SystemInfoData {
            gpu: Some(GpuReadings {
                utilisation: Some(11),
                memory_used: Some(GIB),
                memory_total: Some(0),
                ..silent_gpu()
            }),
            ..SystemInfoData::default()
        };

        let section = graphics_section(&data).expect("graphics window");

        assert_eq!(section.rows.len(), 1);
        assert!(matches!(
            &section.rows[0],
            Row::Meter { label, percent: 11, .. } if label == "Load"
        ));
    }

    #[test]
    fn gpu_memory_reads_as_a_pool_with_its_computed_share() {
        let data = SystemInfoData {
            gpu: Some(GpuReadings {
                memory_used: Some(2 * GIB),
                memory_total: Some(8 * GIB),
                ..silent_gpu()
            }),
            ..SystemInfoData::default()
        };

        let section = graphics_section(&data).expect("graphics window");

        assert!(matches!(
            &section.rows[0],
            Row::Meter { label, value, percent: 25 }
                if label == "Memory" && value == "2.0 / 8.0 GiB (25%)"
        ));
    }

    #[test]
    fn disk_rows_keep_their_mount_and_order() {
        let data = SystemInfoData {
            disks: vec![
                DiskData {
                    mount:         "/".to_owned(),
                    used:          GIB,
                    total:         4 * GIB,
                    usage_percent: 25
                },
                DiskData {
                    mount:         "/home".to_owned(),
                    used:          3 * GIB,
                    total:         4 * GIB,
                    usage_percent: 75
                },
            ],
            ..SystemInfoData::default()
        };

        let rows = storage_section(&data).expect("storage window").rows;

        assert!(matches!(
            &rows[0],
            Row::Meter { label, percent: 25, .. } if label == "/"
        ));
        assert!(matches!(
            &rows[1],
            Row::Meter { label, value, percent: 75 }
                if label == "/home" && value == "3.0 / 4.0 GiB (75%)"
        ));
    }
}
