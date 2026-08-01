//! Optional vendor utilities, for the metrics the kernel keeps to
//! itself.
//!
//! Everything the panel shows comes from the kernel first. A vendor
//! utility is consulted only where the kernel publishes
//! nothing, it is looked up once rather than on every refresh,
//! it runs on a thread of its own so a slow or wedged program
//! never delays the bar, and its absence is an ordinary state
//! that costs nothing and is never logged as a fault.
//!
//! The AMD compute stack ships such a utility as well, and the panel
//! does not run it: the graphics driver already publishes the
//! temperature, the load and the video memory of every AMD part
//! through the kernel, so spawning a process would buy nothing.
//! Adding a vendor is one entry in [`UTILITIES`] together with
//! the function that reads its output. The polling thread itself
//! lives in [`feed`].

mod feed;

use std::path::PathBuf;

pub use feed::{Feed, ProcessRunner, Runner};

use super::catalog::GpuVendor;

/// How long the panel waits between two runs of a vendor utility.
///
/// A utility costs a process, and on a laptop it can keep a card from
/// powering down, so it runs far more rarely than the bar
/// refreshes and the bar shows the last answer in between.
pub const UTILITY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// Metrics a vendor utility reports for one device.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UtilityMetrics {
    pub temperature:  Option<i32>,
    pub utilisation:  Option<u32>,
    pub memory_used:  Option<u64>,
    pub memory_total: Option<u64>
}

impl UtilityMetrics {
    /// Reports whether the utility answered with any usable number.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.temperature.is_none()
            && self.utilisation.is_none()
            && self.memory_used.is_none()
            && self.memory_total.is_none()
    }
}

/// A program the panel may ask for metrics the kernel does not publish.
#[derive(Debug)]
pub struct Utility {
    /// Program name, looked up on `PATH`.
    pub program: &'static str,
    pub args:    &'static [&'static str],
    /// Vendor whose devices the program reports on.
    pub vendor:  GpuVendor,
    /// Turns the standard output of the program into metrics.
    pub parse:   fn(&str) -> Option<UtilityMetrics>
}

/// Utilities the panel knows how to read.
///
/// The NVIDIA driver publishes a monitoring chip only in its recent
/// releases, and never publishes the load, so its query
/// interface is the one way to show those numbers on an
/// otherwise supported machine.
pub const UTILITIES: [Utility; 1] = [Utility {
    program: "nvidia-smi",
    args:    &[
        "--query-gpu=temperature.gpu,utilization.gpu,memory.used,memory.total",
        "--format=csv,noheader,nounits"
    ],
    vendor:  GpuVendor::Nvidia,
    parse:   parse_query_row
}];

/// The utility that reports on a vendor, when the panel knows one.
#[must_use]
pub fn for_vendor(vendor: GpuVendor) -> Option<&'static Utility> {
    UTILITIES.iter().find(|utility| utility.vendor == vendor)
}

/// Path a program resolves to on `PATH`, when it is installed at all.
#[must_use]
pub fn on_path(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|directory| directory.join(program))
            .find(|candidate| candidate.is_file())
    })
}

/// Reads one comma separated row of `temperature, load, memory used,
/// memory installed`.
///
/// Only the first row is read: a machine with two cards answers with
/// one row each, and the panel shows the card it selected
/// rather than a sum. Fields the driver cannot answer arrive as
/// `[N/A]` and stay empty instead of being shown as a zero.
fn parse_query_row(output: &str) -> Option<UtilityMetrics> {
    let row = output.lines().find(|line| !line.trim().is_empty())?;
    let mut fields = row.split(',').map(str::trim);

    let temperature = fields.next().and_then(|value| value.parse().ok());
    let utilisation = fields
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .map(|value| value.min(100));
    let memory_used = fields.next().and_then(parse_mebibytes);
    let memory_total = fields.next().and_then(parse_mebibytes);

    Some(UtilityMetrics {
        temperature,
        utilisation,
        memory_used,
        memory_total
    })
}

/// Turns a count of mebibytes into bytes, the unit the module carries.
fn parse_mebibytes(value: &str) -> Option<u64> {
    value.parse::<u64>().ok().map(|value| value * 1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_row_becomes_metrics() {
        let metrics = parse_query_row("66, 15, 1234, 8192\n").expect("metrics");

        assert_eq!(metrics.temperature, Some(66));
        assert_eq!(metrics.utilisation, Some(15));
        assert_eq!(metrics.memory_used, Some(1234 * 1024 * 1024));
        assert_eq!(metrics.memory_total, Some(8192 * 1024 * 1024));
    }

    #[test]
    fn only_the_first_card_of_the_answer_is_read() {
        let metrics = parse_query_row("66, 15, 1234, 8192\n71, 40, 512, 8192\n").expect("metrics");

        assert_eq!(metrics.temperature, Some(66));
    }

    #[test]
    fn a_field_the_driver_cannot_answer_stays_empty() {
        let metrics = parse_query_row("[N/A], 15, [N/A], 8192").expect("metrics");

        assert_eq!(metrics.temperature, None);
        assert_eq!(metrics.utilisation, Some(15));
        assert_eq!(metrics.memory_used, None);
        assert_eq!(metrics.memory_total, Some(8192 * 1024 * 1024));
    }

    #[test]
    fn an_empty_answer_reports_nothing() {
        assert_eq!(parse_query_row(""), None);
        assert!(
            parse_query_row("[N/A], [N/A], [N/A], [N/A]")
                .expect("metrics")
                .is_empty()
        );
    }
}
