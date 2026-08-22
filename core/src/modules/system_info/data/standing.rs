//! How the machine is standing: how long, how hard, how loudly.
//!
//! Everything here is read straight out of the kernel's own files, which cost
//! a read each and answer without a round trip to anything. None of it moves
//! fast enough to need damping: an uptime counts seconds, a load average is
//! already averaged over a minute, and a fan is reported in whole revolutions.

use std::{fs, time::Duration};

/// How long the machine has been up.
#[must_use]
pub fn uptime() -> Option<Duration> {
    let read = fs::read_to_string("/proc/uptime").ok()?;
    let seconds: f64 = read.split_whitespace().next()?.parse().ok()?;

    (seconds > 0.0).then(|| Duration::from_secs_f64(seconds))
}

/// What the kernel says the last minute, five and fifteen were like.
///
/// A load average is runnable tasks, not a share of anything: on a machine of
/// sixteen threads a load of sixteen is full, and a load of one is a machine
/// with one thing to do.
#[must_use]
pub fn load() -> Option<(f32, f32, f32)> {
    let read = fs::read_to_string("/proc/loadavg").ok()?;
    let mut fields = read.split_whitespace();

    Some((
        fields.next()?.parse().ok()?,
        fields.next()?.parse().ok()?,
        fields.next()?.parse().ok()?
    ))
}

/// How many tasks the kernel is holding, and how many of them want a core.
#[must_use]
pub fn tasks() -> Option<(u32, u32)> {
    let read = fs::read_to_string("/proc/loadavg").ok()?;

    parse_tasks(&read)
}

/// Reads the runnable and total task counts out of a load average line.
fn parse_tasks(line: &str) -> Option<(u32, u32)> {
    let field = line.split_whitespace().nth(3)?;
    let (running, total) = field.split_once('/')?;

    Some((running.parse().ok()?, total.parse().ok()?))
}

/// Every fan the machine reports, with what it is called and how fast it runs.
///
/// A fan reporting zero is a fan that is not turning, which is worth saying:
/// a silent machine and a machine whose fan died read the same on a
/// temperature alone.
#[must_use]
pub fn fans() -> Vec<(String, u32)> {
    let Ok(chips) = fs::read_dir("/sys/class/hwmon") else {
        return Vec::new();
    };

    let mut found: Vec<(String, u32)> = chips
        .flatten()
        .flat_map(|chip| fans_of(&chip.path()))
        .collect();

    found.sort_by(|left, right| left.0.cmp(&right.0));
    found
}

/// The fans one monitoring chip reports.
fn fans_of(chip: &std::path::Path) -> Vec<(String, u32)> {
    let Ok(entries) = fs::read_dir(chip) else {
        return Vec::new();
    };

    let chip_name = fs::read_to_string(chip.join("name"))
        .map(|name| name.trim().to_owned())
        .unwrap_or_default();

    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let file = path.file_name()?.to_str()?.to_owned();
            let index = file.strip_prefix("fan")?.strip_suffix("_input")?;
            let rpm: u32 = fs::read_to_string(&path).ok()?.trim().parse().ok()?;
            let label = fs::read_to_string(chip.join(format!("fan{index}_label")))
                .ok()
                .map(|label| label.trim().to_owned())
                .filter(|label| !label.is_empty())
                .unwrap_or_else(|| named(&chip_name, index));

            Some((label, rpm))
        })
        .collect()
}

/// The name a fan gets when its driver labels it with nothing.
fn named(chip: &str, index: &str) -> String {
    if chip.is_empty() {
        return format!("fan {index}");
    }

    format!("{chip} fan {index}")
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_task_counts_are_read_out_of_the_load_average_line() {
        assert_eq!(parse_tasks("0.52 0.58 0.59 3/1234 5678"), Some((3, 1234)));
    }

    #[test]
    fn a_line_that_is_not_one_reads_as_nothing() {
        assert!(parse_tasks("").is_none());
        assert!(parse_tasks("0.52 0.58 0.59").is_none());
        assert!(parse_tasks("0.52 0.58 0.59 nonsense 1").is_none());
    }

    #[test]
    fn an_unlabelled_fan_is_named_after_its_chip() {
        assert_eq!(named("amdgpu", "1"), "amdgpu fan 1");
        assert_eq!(named("", "1"), "fan 1");
    }

    #[test]
    fn the_machine_answers_how_long_it_has_been_up() {
        // read from the kernel on any machine these tests run on; a machine
        // without the file is not one this bar runs on
        assert!(uptime().is_some_and(|up| up.as_secs() > 0));
    }

    #[test]
    fn the_load_average_reads_as_three_figures() {
        let read = load().expect("the kernel reports a load average");

        assert!(read.0 >= 0.0 && read.1 >= 0.0 && read.2 >= 0.0);
    }
}
