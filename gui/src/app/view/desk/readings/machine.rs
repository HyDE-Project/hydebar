//! The hardware the session runs on, read off one system sample.

use hydebar_core::modules::system_info::{DiskData, SystemInfoData, gigabytes, used_of_total};

use super::{Panel, push};

/// The machine itself: what it runs and what steers it.
pub fn system(data: &SystemInfoData) -> Option<Panel> {
    let mut rows = Vec::new();

    push(&mut rows, "kernel", data.kernel.clone());
    push(&mut rows, "processor", data.cpu_model.clone());
    push(
        &mut rows,
        "cores",
        data.cpu_cores
            .map(|cores| format!("{cores} / {} threads", data.cpu_count))
    );
    push(&mut rows, "governor", data.cpu_governor.clone());
    push(&mut rows, "microcode", data.cpu_microcode.clone());

    Panel::of("system", rows)
}

/// The link: where it lands and what is crossing it.
pub fn network(data: &SystemInfoData) -> Option<Panel> {
    let network = data.network.as_ref()?;

    Panel::of(
        "network",
        vec![
            ("address".to_owned(), network.ip.clone()),
            (
                "down".to_owned(),
                format!("{} KB/s", network.download_speed)
            ),
            ("up".to_owned(), format!("{} KB/s", network.upload_speed)),
        ]
    )
}

/// The processor: what it is, how hard it is working, how hot and how fast.
pub fn processor(data: &SystemInfoData) -> Option<Panel> {
    let mut rows = vec![("load".to_owned(), format!("{}%", data.cpu_usage))];

    push(&mut rows, "model", data.cpu_model.clone());
    push(
        &mut rows,
        "cores",
        data.cpu_cores
            .map(|cores| format!("{cores} / {} threads", data.cpu_count))
    );
    push(
        &mut rows,
        "clock",
        data.cpu_current_mhz.map(|mhz| {
            data.cpu_max_mhz.map_or_else(
                || format!("{:.2} GHz", f64::from(mhz) / 1000.0),
                |max| {
                    format!(
                        "{:.2} / {:.2} GHz",
                        f64::from(mhz) / 1000.0,
                        f64::from(max) / 1000.0
                    )
                }
            )
        })
    );
    push(&mut rows, "governor", data.cpu_governor.clone());
    push(
        &mut rows,
        "temperature",
        data.cpu_temperature.map(|degrees| format!("{degrees}°C"))
    );

    Panel::of("processor", rows)
}

/// The processor temperature on its own, with the sensor it is read from.
pub fn cpu_temperature(data: &SystemInfoData) -> Option<Panel> {
    let mut rows = Vec::new();

    push(
        &mut rows,
        "temperature",
        data.cpu_temperature.map(|degrees| format!("{degrees}°C"))
    );
    push(&mut rows, "sensor", data.cpu_temperature_source.clone());

    Panel::of("cpu temperature", rows)
}

/// The graphics device, on a machine that reports one.
pub fn graphics(data: &SystemInfoData) -> Option<Panel> {
    let gpu = data.gpu.as_ref()?;
    let mut rows = vec![("driver".to_owned(), gpu.name.clone())];

    push(&mut rows, "sensor", gpu.source.clone());

    push(
        &mut rows,
        "load",
        gpu.utilisation.map(|share| format!("{share}%"))
    );
    push(
        &mut rows,
        "temperature",
        gpu.temperature.map(|degrees| format!("{degrees}°C"))
    );
    push(
        &mut rows,
        "memory",
        gpu.memory_used
            .zip(gpu.memory_total)
            .map(|(used, total)| used_of_total(used, total))
    );

    Panel::of("graphics", rows)
}

/// Memory and swap, each in use against what there is.
pub fn memory(data: &SystemInfoData) -> Option<Panel> {
    let mut rows = vec![
        (
            "in use".to_owned(),
            format!(
                "{} ({}%)",
                used_of_total(data.memory_used, data.memory_total),
                data.memory_usage
            )
        ),
        (
            "available".to_owned(),
            format!(
                "{} GiB",
                gigabytes(data.memory_total.saturating_sub(data.memory_used))
            )
        ),
        (
            "cached".to_owned(),
            format!("{} GiB", gigabytes(data.memory_cached))
        ),
    ];

    if data.memory_swap_total > 0 {
        rows.push((
            "swap".to_owned(),
            format!(
                "{} ({}%)",
                used_of_total(data.memory_swap_used, data.memory_swap_total),
                data.memory_swap_usage
            )
        ));
        push(&mut rows, "backend", data.swap_backend.clone());
    }

    Panel::of("memory", rows)
}

/// Every filesystem, with what is left on it.
///
/// One line per filesystem rather than per mount point: a machine laid out
/// in subvolumes mounts the same filesystem a dozen times over — `/`,
/// `/home`, `/var/log` and the rest of them — and each of those mounts
/// reports the same bytes. Listing them all would fill the column with one
/// number repeated. The shortest mount stands for the filesystem, because
/// that is the one the user thinks of it by.
pub fn storage(data: &SystemInfoData) -> Option<Panel> {
    let mut filesystems: Vec<&DiskData> = Vec::new();

    for disk in &data.disks {
        match filesystems
            .iter_mut()
            .find(|seen| seen.used == disk.used && seen.total == disk.total)
        {
            Some(seen) => {
                if disk.mount.len() < seen.mount.len() {
                    *seen = disk;
                }
            }
            None => filesystems.push(disk)
        }
    }

    Panel::of(
        "storage",
        filesystems
            .into_iter()
            .map(|disk| (disk.mount.clone(), used_of_total(disk.used, disk.total)))
            .collect()
    )
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_core::modules::system_info::NetworkData;

    use super::*;

    /// A machine that reported nothing at all, filled in per test.
    ///
    /// Deliberately not a real sample: a reading taken off the machine the
    /// tests run on would assert about whatever hardware that is.
    fn sample() -> SystemInfoData {
        SystemInfoData::default()
    }

    #[test]
    fn a_machine_that_names_nothing_draws_no_system_panel() {
        assert_eq!(system(&sample()), None);
    }

    #[test]
    fn a_machine_that_names_itself_is_read_top_to_bottom() {
        let mut data = sample();
        data.kernel = Some("7.1.8".to_owned());
        data.cpu_cores = Some(6);
        data.cpu_count = 12;

        let panel = system(&data).expect("a machine that answered");

        assert_eq!(panel.title, "system");
        assert_eq!(panel.rows[0], ("kernel".to_owned(), "7.1.8".to_owned()));
        assert_eq!(
            panel.rows[1],
            ("cores".to_owned(), "6 / 12 threads".to_owned())
        );
    }

    #[test]
    fn the_link_is_drawn_only_once_it_answers() {
        let mut data = sample();
        data.network = None;
        assert_eq!(network(&data), None);

        data.network = Some(NetworkData::new(
            "192.168.1.2".to_owned(),
            120,
            17,
            std::time::Instant::now()
        ));

        let panel = network(&data).expect("a link that answered");
        assert_eq!(panel.rows.len(), 3);
        assert_eq!(panel.rows[0].1, "192.168.1.2");
    }

    #[test]
    fn a_machine_without_swap_says_nothing_about_it() {
        let mut data = sample();
        data.memory_swap_total = 0;

        let panel = memory(&data).expect("memory is always reported");

        assert!(!panel.rows.iter().any(|(label, _)| label == "swap"));
    }

    #[test]
    fn every_mount_gets_its_own_line() {
        let mut data = sample();
        data.disks = vec![
            DiskData {
                mount:         "/".to_owned(),
                used:          1024 * 1024 * 1024,
                total:         2 * 1024 * 1024 * 1024,
                usage_percent: 50
            },
            DiskData {
                mount:         "/home".to_owned(),
                used:          1024 * 1024 * 1024,
                total:         4 * 1024 * 1024 * 1024,
                usage_percent: 25
            },
        ];

        let panel = storage(&data).expect("two mounts");

        assert_eq!(panel.rows.len(), 2);
        assert_eq!(
            panel.rows[1],
            ("/home".to_owned(), "1.0 / 4.0 GiB".to_owned())
        );
    }

    #[test]
    fn one_filesystem_mounted_many_times_takes_one_line() {
        let mut data = sample();
        data.disks = ["/var/log", "/", "/home"]
            .into_iter()
            .map(|mount| DiskData {
                mount:         mount.to_owned(),
                used:          1024 * 1024 * 1024,
                total:         2 * 1024 * 1024 * 1024,
                usage_percent: 50
            })
            .collect();

        let panel = storage(&data).expect("one filesystem");

        assert_eq!(panel.rows.len(), 1);
        assert_eq!(panel.rows[0].0, "/");
    }

    #[test]
    fn an_unmounted_machine_draws_no_storage_panel() {
        let mut data = sample();
        data.disks = Vec::new();

        assert_eq!(storage(&data), None);
    }

    #[test]
    fn the_processor_always_states_its_load() {
        let mut data = sample();
        data.cpu_usage = 42;
        data.cpu_temperature = None;
        data.cpu_current_mhz = None;

        let panel = processor(&data).expect("the load is always known");

        assert_eq!(panel.rows, vec![("load".to_owned(), "42%".to_owned())]);
    }
}
