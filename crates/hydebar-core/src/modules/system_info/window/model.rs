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
pub const fn meter_level(percent: u32) -> MeterLevel {
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
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "the ratio times 100 is bounded to 0..=100 and fits u32; f64 blurs only fractions below the whole percent shown"
)]
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

/// A frequency stated in MHz, spelled in GHz.
fn gigahertz(mhz: u32) -> String {
    format!("{:.2}", f64::from(mhz) / 1000.0)
}

/// The one section a scoped window shows, picked by its icon.
///
/// The standalone entries open a window of their own subject alone;
/// each subject is built by its own constructor, so the window costs
/// its own rows and nothing of its neighbours'.
#[must_use]
pub fn scoped_section(
    data: &SystemInfoData,
    icon: crate::components::icons::Icons
) -> Option<Section> {
    match icon {
        crate::components::icons::Icons::Cpu => Some(processor_section_full(data)),
        crate::components::icons::Icons::Mem => Some(memory_section(data)),
        crate::components::icons::Icons::Gpu => graphics_section(data),
        crate::components::icons::Icons::Drive => storage_section(data),
        crate::components::icons::Icons::Ethernet => network_section(data),
        _ => None
    }
}

/// The processor window of the standalone processor entry.
///
/// The monitor's processor section minus the temperature: the
/// temperature entry owns that reading, and each window shows its
/// own subject and nothing of its neighbours'.
#[must_use]
pub fn processor_section(data: &SystemInfoData) -> Option<Section> {
    let mut section = scoped_section(data, crate::components::icons::Icons::Cpu)?;

    section
        .rows
        .retain(|row| !matches!(row, Row::Fact { label, .. } if label.starts_with("Temperature")));

    Some(section)
}

/// The window of the standalone processor temperature entry.
#[must_use]
pub fn cpu_temperature_section(data: &SystemInfoData) -> Option<Section> {
    let temperature = data.cpu_temperature?;

    Some(Section {
        icon:  crate::components::icons::Icons::Temp,
        title: "CPU temperature",
        note:  data.cpu_temperature_source.clone(),
        rows:  vec![fact("Temperature", format!("{temperature}°C"))]
    })
}

/// Rows of the processor section, in drawing order.
fn processor_rows(data: &SystemInfoData) -> Vec<Row> {
    let mut processor = vec![meter(
        "Load",
        format!("{}%", data.cpu_usage),
        data.cpu_usage
    )];

    match (data.cpu_cores, data.cpu_count) {
        (Some(cores), count) if count > 0 => {
            processor.push(fact("Cores", format!("{cores} ({count} threads)")));
        }
        (None, count) if count > 0 => {
            processor.push(fact("Threads", count.to_string()));
        }
        _ => {}
    }

    match (data.cpu_current_mhz, data.cpu_max_mhz) {
        (Some(current), Some(max)) => {
            processor.push(fact(
                "Frequency",
                format!("{} / {} GHz", gigahertz(current), gigahertz(max))
            ));
        }
        (Some(current), None) => {
            processor.push(fact("Frequency", format!("{} GHz", gigahertz(current))));
        }
        (None, Some(max)) => {
            processor.push(fact("Max frequency", format!("{} GHz", gigahertz(max))));
        }
        (None, None) => {}
    }

    if let Some(governor) = data.cpu_governor.as_ref() {
        processor.push(fact("Governor", governor.clone()));
    }

    if let Some(temperature) = data.cpu_temperature {
        let label = data.cpu_temperature_source.as_ref().map_or_else(
            || "Temperature".to_owned(),
            |source| format!("Temperature ({source})")
        );

        processor.push(fact(&label, format!("{temperature}°C")));
    }

    if let Some(microcode) = data.cpu_microcode.as_ref() {
        processor.push(fact("Microcode", microcode.clone()));
    }

    if let Some(kernel) = data.kernel.as_ref() {
        processor.push(fact("Kernel", kernel.clone()));
    }

    processor
}

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
                super::super::view::gigabytes(data.memory_total.saturating_sub(data.memory_used))
            )
        ));
    }

    if data.memory_cached > 0 {
        memory.push(fact(
            "Cached",
            format!("{} GiB", super::super::view::gigabytes(data.memory_cached))
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

/// The processor section of the monitor.
#[must_use]
pub fn processor_section_full(data: &SystemInfoData) -> Section {
    Section {
        icon:  Icons::Cpu,
        title: "Processor",
        note:  data.cpu_model.clone(),
        rows:  processor_rows(data)
    }
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
        note: Some(gpu_title(gpu)),
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
            fact("Download", format_speed(network.download_speed)),
            fact("Upload", format_speed(network.upload_speed)),
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
    let mut sections = vec![processor_section_full(data), memory_section(data)];

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
        DiskData, SystemInfoData,
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
    fn a_share_is_truncated_not_rounded() {
        assert_eq!(share(1, 3), 33);
        assert_eq!(share(2, 3), 66);
    }

    #[test]
    fn a_pool_fuller_than_itself_reads_over_a_hundred() {
        assert_eq!(share(3, 2), 150);
    }

    #[test]
    fn each_scoped_icon_opens_its_own_subject() {
        let data = SystemInfoData::default();

        let processor = scoped_section(&data, Icons::Cpu).expect("processor window");
        let memory = scoped_section(&data, Icons::Mem).expect("memory window");

        assert_eq!(processor.title, "Processor");
        assert_eq!(memory.title, "Memory");
        assert!(scoped_section(&data, Icons::Temp).is_none());
    }

    #[test]
    fn a_scoped_window_on_a_missing_subject_stays_shut() {
        let data = SystemInfoData::default();

        assert!(scoped_section(&data, Icons::Gpu).is_none());
        assert!(scoped_section(&data, Icons::Drive).is_none());
        assert!(scoped_section(&data, Icons::Ethernet).is_none());
    }

    #[test]
    fn a_machine_before_its_first_sample_states_only_its_load() {
        let rows = processor_section_full(&SystemInfoData::default()).rows;

        assert_eq!(rows.len(), 1);
        assert!(matches!(
            &rows[0],
            Row::Meter { label, value, percent: 0 } if label == "Load" && value == "0%"
        ));
    }

    #[test]
    fn cores_and_threads_read_as_one_fact() {
        let data = SystemInfoData {
            cpu_cores: Some(8),
            cpu_count: 16,
            ..SystemInfoData::default()
        };

        let rows = processor_section_full(&data).rows;

        assert!(matches!(
            row_with_label(&rows, "Cores"),
            Some(Row::Fact { value, .. }) if value == "8 (16 threads)"
        ));
    }

    #[test]
    fn the_frequency_row_pairs_current_with_max_in_gigahertz() {
        let data = SystemInfoData {
            cpu_current_mhz: Some(4550),
            cpu_max_mhz: Some(5759),
            ..SystemInfoData::default()
        };

        let rows = processor_section_full(&data).rows;

        assert!(matches!(
            row_with_label(&rows, "Frequency"),
            Some(Row::Fact { value, .. }) if value == "4.55 / 5.76 GHz"
        ));
    }

    #[test]
    fn a_machine_reporting_only_its_ceiling_says_max_frequency() {
        let data = SystemInfoData {
            cpu_max_mhz: Some(5000),
            ..SystemInfoData::default()
        };

        let rows = processor_section_full(&data).rows;

        assert!(row_with_label(&rows, "Frequency").is_none());
        assert!(matches!(
            row_with_label(&rows, "Max frequency"),
            Some(Row::Fact { value, .. }) if value == "5.00 GHz"
        ));
    }

    #[test]
    fn a_current_frequency_alone_still_reads_in_gigahertz() {
        let data = SystemInfoData {
            cpu_current_mhz: Some(4550),
            ..SystemInfoData::default()
        };

        let rows = processor_section_full(&data).rows;

        assert!(matches!(
            row_with_label(&rows, "Frequency"),
            Some(Row::Fact { value, .. }) if value == "4.55 GHz"
        ));
    }

    #[test]
    fn governor_microcode_and_kernel_each_take_a_row() {
        let data = SystemInfoData {
            cpu_governor: Some("schedutil".to_owned()),
            cpu_microcode: Some("0xb404032".to_owned()),
            kernel: Some("7.1.5-1-cachyos".to_owned()),
            ..SystemInfoData::default()
        };

        let rows = processor_section_full(&data).rows;

        assert!(matches!(
            row_with_label(&rows, "Governor"),
            Some(Row::Fact { value, .. }) if value == "schedutil"
        ));
        assert!(matches!(
            row_with_label(&rows, "Microcode"),
            Some(Row::Fact { value, .. }) if value == "0xb404032"
        ));
        assert!(matches!(
            row_with_label(&rows, "Kernel"),
            Some(Row::Fact { value, .. }) if value == "7.1.5-1-cachyos"
        ));
    }

    #[test]
    fn the_temperature_row_names_its_sensor_inline() {
        let data = SystemInfoData {
            cpu_temperature: Some(56),
            cpu_temperature_source: Some("k10temp Tctl".to_owned()),
            ..SystemInfoData::default()
        };

        let rows = processor_section_full(&data).rows;

        assert!(matches!(
            row_with_label(&rows, "Temperature (k10temp Tctl)"),
            Some(Row::Fact { value, .. }) if value == "56°C"
        ));
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
