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
/// the icon is the section's stable identity, so the window and the
/// full monitor can never disagree about the rows.
#[must_use]
pub fn scoped_section(
    data: &SystemInfoData,
    icon: crate::components::icons::Icons
) -> Option<Section> {
    sections(data).into_iter().find(|section| section.icon == icon)
}

/// The processor window of the standalone processor entry.
///
/// The monitor's processor section minus the temperature: the
/// temperature entry owns that reading, and each window shows its
/// own subject and nothing of its neighbours'.
#[must_use]
pub fn processor_section(data: &SystemInfoData) -> Option<Section> {
    let mut section = scoped_section(data, crate::components::icons::Icons::Cpu)?;

    section.rows.retain(|row| {
        !matches!(row, Row::Fact { label, .. } if label.starts_with("Temperature"))
    });

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
        let label = match data.cpu_temperature_source.as_ref() {
            Some(source) => format!("Temperature ({source})"),
            None => "Temperature".to_owned()
        };

        processor.push(fact(&label, format!("{temperature}°C")));
    }

    if let Some(microcode) = data.cpu_microcode.as_ref() {
        processor.push(fact("Microcode", microcode.clone()));
    }

    if let Some(kernel) = data.kernel.as_ref() {
        processor.push(fact("Kernel", kernel.clone()));
    }

    sections.push(Section {
        icon:  Icons::Cpu,
        title: "Processor",
        note:  data.cpu_model.clone(),
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

    if data.memory_cached > 0 {
        memory.push(fact(
            "Cached",
            format!(
                "{} GiB",
                super::super::view::gigabytes(data.memory_cached)
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

        if let Some(backend) = data.swap_backend.as_ref() {
            memory.push(fact("Swap device", backend.clone()));
        }
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
