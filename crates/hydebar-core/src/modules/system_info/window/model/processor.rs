//! The processor's own windows: the monitor section, the standalone
//! entry and the temperature window beside it.

use super::{
    super::super::data::SystemInfoData,
    Row, Section, fact, gigahertz, meter
};
use crate::components::icons::Icons;

/// The one section a scoped window shows, picked by its icon.
///
/// The standalone entries open a window of their own subject alone;
/// each subject is built by its own constructor, so the window costs
/// its own rows and nothing of its neighbours'.
#[must_use]
pub fn scoped_section(data: &SystemInfoData, icon: Icons) -> Option<Section> {
    match icon {
        Icons::Cpu => Some(processor_section_full(data)),
        Icons::Mem => Some(super::memory_section(data)),
        Icons::Gpu => super::graphics_section(data),
        Icons::Drive => super::storage_section(data),
        Icons::Ethernet => super::network_section(data),
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
    let mut section = scoped_section(data, Icons::Cpu)?;

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
        icon:  Icons::Temp,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
