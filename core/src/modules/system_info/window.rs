//! The system monitor window: what it says, how much room it needs, and
//! how it is drawn.
//!
//! Three rooms after the calendar's pattern: [`model`] states every
//! section and row out of one sample, [`metrics`] measures the room that
//! statement needs from the same constants the drawing uses, and
//! [`render`] draws it. The window is measured rather than stock-sized,
//! so the box hugs the readouts this machine actually reports.

pub(super) mod model;

pub(super) mod metrics;

mod render;

pub use metrics::{content_height_of, content_width, section_window_height};
pub use render::{build_menu_view, build_section_window};

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_proto::config::{SystemIndicator, SystemModuleConfig};

    use super::{
        metrics,
        model::{self, MeterLevel, Row}
    };
    use crate::modules::system_info::{
        DiskData, SystemInfoData,
        sensors::{GpuPlacement, GpuReadings, GpuVendor}
    };

    const GIB: u64 = 1024 * 1024 * 1024;

    fn machine() -> SystemInfoData {
        SystemInfoData {
            cpu_usage: 34,
            cpu_count: 32,
            memory_usage: 15,
            memory_used: 9 * GIB,
            memory_total: 62 * GIB,
            memory_swap_usage: 6,
            memory_swap_used: GIB / 2,
            memory_swap_total: 8 * GIB,
            cpu_temperature: Some(56),
            gpu: Some(GpuReadings {
                name:         "amdgpu".to_owned(),
                source:       Some("amdgpu junction".to_owned()),
                vendor:       GpuVendor::Amd,
                placement:    GpuPlacement::Discrete,
                temperature:  Some(47),
                utilisation:  Some(11),
                memory_used:  Some(GIB),
                memory_total: Some(8 * GIB)
            }),
            disks: vec![DiskData {
                mount:         "/".to_owned(),
                used:          213 * GIB,
                total:         476 * GIB,
                usage_percent: 44
            }],
            network: Some(crate::modules::system_info::NetworkData::new(
                "192.168.1.5".to_owned(),
                1500,
                120,
                std::time::Instant::now()
            )),
            ..SystemInfoData::default()
        }
    }

    fn titles(data: &SystemInfoData) -> Vec<&'static str> {
        model::sections(data)
            .into_iter()
            .map(|section| section.title)
            .collect()
    }

    #[test]
    fn meters_change_colour_at_the_stated_shares() {
        assert_eq!(model::meter_level(0), MeterLevel::Calm);
        assert_eq!(model::meter_level(79), MeterLevel::Calm);
        assert_eq!(model::meter_level(80), MeterLevel::Busy);
        assert_eq!(model::meter_level(94), MeterLevel::Busy);
        assert_eq!(model::meter_level(95), MeterLevel::Critical);
        assert_eq!(model::meter_level(100), MeterLevel::Critical);
    }

    #[test]
    fn a_share_is_a_percentage_and_an_empty_pool_has_none() {
        assert_eq!(model::share(GIB, 8 * GIB), 12);
        assert_eq!(model::share(0, 8 * GIB), 0);
        assert_eq!(model::share(GIB, 0), 0);
    }

    #[test]
    fn a_full_machine_states_every_section_in_order() {
        assert_eq!(
            titles(&machine()),
            vec!["Processor", "Memory", "Graphics", "Storage", "Network"]
        );
    }

    #[test]
    fn a_machine_without_swap_shows_no_swap_row() {
        let mut data = machine();
        data.memory_swap_total = 0;

        let sections = model::sections(&data);
        let memory = sections
            .iter()
            .find(|section| section.title == "Memory")
            .expect("memory section");

        assert_eq!(memory.rows.len(), 2);
        assert!(matches!(
            &memory.rows[1],
            Row::Fact { label, value } if label == "Available" && value == "53.0 GiB"
        ));
    }

    #[test]
    fn swap_reads_as_a_pool_when_the_machine_has_one() {
        let sections = model::sections(&machine());
        let memory = sections
            .iter()
            .find(|section| section.title == "Memory")
            .expect("memory section");

        assert!(matches!(
            &memory.rows[2],
            Row::Meter { label, value, percent: 6 }
                if label == "Swap" && value == "0.5 / 8.0 GiB (6%)"
        ));
    }

    /// The window says how many threads the load percentage averages
    /// over, so `3%` on a 32-thread machine reads as the light load it
    /// is.
    #[test]
    fn the_processor_section_counts_its_threads() {
        let sections = model::sections(&machine());
        let processor = sections
            .iter()
            .find(|section| section.title == "Processor")
            .expect("processor section");

        assert!(matches!(
            &processor.rows[1],
            Row::Fact { label, value } if label == "Threads" && value == "32"
        ));
    }

    #[test]
    fn the_processor_window_leaves_the_temperature_to_its_own_entry() {
        let mut data = machine();
        data.cpu_temperature_source = Some("k10temp Tctl".to_owned());

        let section = model::processor_section(&data).expect("processor section");

        assert!(
            !section.rows.iter().any(|row| matches!(
                row,
                model::Row::Fact { label, .. } if label.starts_with("Temperature")
            )),
            "the temperature row belongs to the temperature window"
        );
    }

    #[test]
    fn the_temperature_window_names_its_sensor() {
        let mut data = machine();
        data.cpu_temperature_source = Some("k10temp Tctl".to_owned());

        let section = model::cpu_temperature_section(&data).expect("temperature section");

        assert_eq!(section.title, "CPU temperature");
        assert_eq!(section.note.as_deref(), Some("k10temp Tctl"));
        assert!(matches!(
            &section.rows[0],
            model::Row::Fact { label, value } if label == "Temperature" && value == "56°C"
        ));
    }

    #[test]
    fn a_machine_without_a_cpu_sensor_has_no_temperature_window() {
        let mut data = machine();
        data.cpu_temperature = None;

        assert!(model::cpu_temperature_section(&data).is_none());
    }

    #[test]
    fn the_graphics_section_names_its_source() {
        let sections = model::sections(&machine());
        let graphics = sections
            .iter()
            .find(|section| section.title == "Graphics")
            .expect("graphics section");

        assert_eq!(graphics.note.as_deref(), Some("Graphics (amdgpu junction)"));
        assert_eq!(graphics.rows.len(), 3);
    }

    #[test]
    fn transfer_rates_read_in_the_window_as_on_the_bar() {
        let sections = model::sections(&machine());
        let network = sections
            .iter()
            .find(|section| section.title == "Network")
            .expect("network section");

        assert!(matches!(
            &network.rows[1],
            Row::Fact { label, value } if label == "Download" && value == "1.5 MB/s"
        ));
    }

    #[test]
    fn a_machine_reporting_nothing_still_states_load_and_memory() {
        assert_eq!(
            titles(&SystemInfoData::default()),
            vec!["Processor", "Memory"]
        );
    }

    #[test]
    fn footnotes_name_what_the_machine_cannot_report() {
        let footnotes =
            model::footnotes(&SystemInfoData::default(), &SystemModuleConfig::default());

        assert!(
            footnotes
                .iter()
                .any(|line| line.starts_with("CPU temperature"))
        );
        assert!(footnotes.iter().any(|line| line.starts_with("Swap usage")));
    }

    #[test]
    fn a_listed_disk_that_is_not_mounted_is_a_footnote() {
        let config = SystemModuleConfig {
            indicators: vec![SystemIndicator::Disk("/data".to_owned())],
            ..SystemModuleConfig::default()
        };

        let footnotes = model::footnotes(&machine(), &config);

        assert!(footnotes.iter().any(|line| line.starts_with("Disk /data")));
    }

    #[test]
    fn the_window_grows_with_what_it_shows() {
        let config = SystemModuleConfig::default();
        let measure = |data: &SystemInfoData| {
            metrics::content_height_of(&model::sections(data), &model::footnotes(data, &config))
        };
        let bare = measure(&SystemInfoData::default());
        let full = measure(&machine());

        assert!(full > bare);
        assert!(bare > 0.0);
        assert!(metrics::content_width(10.0) > 0.0);
    }
}
