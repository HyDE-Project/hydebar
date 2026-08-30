//! The hardware the session runs on, read off one system sample.
//!
//! Three rooms, by what the reading is of: [`chip`] is the two processors and
//! what they are doing, [`store`] is where the machine keeps things — its
//! memory, its swap, its filesystems — and here is the machine itself, how
//! long it has been up and how loudly it is working.

mod chip;
mod store;

pub use chip::{cpu_temperature, graphics, processor};
use hydebar_core::modules::system_info::SystemInfoData;
pub use store::{memory, network, storage};

use super::{Panel, push};

/// The processor has a block of its own, so this one names what stands
/// behind it rather than repeating it: the kernel the session runs and the
/// firmware revision under it.
pub fn system(data: &SystemInfoData) -> Option<Panel> {
    let mut rows = Vec::new();

    push(&mut rows, "kernel", data.kernel.clone());
    push(&mut rows, "microcode", data.cpu_microcode.clone());
    push(&mut rows, "up", data.uptime.map(spelled));
    push(
        &mut rows,
        "load",
        data.load
            .map(|(minute, five, quarter)| format!("{minute:.2} · {five:.2} · {quarter:.2}"))
    );
    push(
        &mut rows,
        "tasks",
        data.tasks
            .map(|(running, held)| format!("{running} running of {held}"))
    );

    Panel::of("system", rows)
}

/// How long the machine has been up, said the way a person says it.
///
/// Days and hours on a machine that has been up for days, hours and minutes
/// below that, and minutes alone in the first hour: the second a machine came
/// up on is never the thing being asked about.
fn spelled(uptime: std::time::Duration) -> String {
    let minutes = uptime.as_secs() / 60;
    let (days, hours, minutes) = (minutes / 1440, (minutes % 1440) / 60, minutes % 60);

    match (days, hours) {
        (0, 0) => format!("{minutes}m"),
        (0, hours) => format!("{hours}h {minutes:02}m"),
        (days, hours) => format!("{days}d {hours:02}h")
    }
}

/// The fans, and how fast each of them is turning.
///
/// A fan reporting nothing is left in: a machine that is silent because it is
/// cool and one that is silent because a fan stopped read the same on a
/// temperature alone, and only this tells them apart.
pub fn cooling(data: &SystemInfoData) -> Option<Panel> {
    Panel::of(
        "cooling",
        data.fans
            .iter()
            .map(|(name, rpm)| {
                (
                    name.clone(),
                    if *rpm == 0 {
                        "stopped".to_owned()
                    } else {
                        format!("{rpm} rpm")
                    }
                )
            })
            .collect()
    )
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_core::modules::system_info::{DiskData, NetworkData};

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
        data.cpu_microcode = Some("0xb7".to_owned());

        let panel = system(&data).expect("a machine that answered");

        assert_eq!(panel.title, "system");
        assert_eq!(panel.rows[0], ("kernel".to_owned(), "7.1.8".to_owned()));
        assert_eq!(panel.rows[1], ("microcode".to_owned(), "0xb7".to_owned()));
    }

    #[test]
    fn the_machine_block_leaves_the_processor_to_its_own() {
        let mut data = sample();
        data.kernel = Some("7.1.8".to_owned());
        data.cpu_model = Some("a processor".to_owned());
        data.cpu_cores = Some(6);
        data.cpu_governor = Some("powersave".to_owned());

        let panel = system(&data).expect("a machine that answered");

        for repeated in ["processor", "cores", "governor"] {
            assert!(
                !panel.rows.iter().any(|(label, _)| label == repeated),
                "{repeated} is stated by the processor block, not this one"
            );
        }
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
        assert_eq!(panel.rows.len(), 5, "address, both speeds and both totals");
        assert_eq!(panel.rows[0].1, "192.168.1.2");
    }

    #[test]
    fn a_machine_without_swap_says_nothing_about_it() {
        let mut data = sample();
        data.memory_swap_total = 0;

        let panel = memory(&data, &crate::app::state::history::Trail::default())
            .expect("memory is always reported");

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

        let panel = processor(&data, &crate::app::state::history::Trail::default())
            .expect("the load is always known");

        assert_eq!(panel.rows, vec![("load".to_owned(), "42%".to_owned())]);
    }
}
