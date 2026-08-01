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
//! the function that reads its output.

use std::{
    io,
    path::PathBuf,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering}
    },
    thread,
    time::Duration
};

use log::debug;

use super::catalog::GpuVendor;

/// How long the panel waits between two runs of a vendor utility.
///
/// A utility costs a process, and on a laptop it can keep a card from
/// powering down, so it runs far more rarely than the bar
/// refreshes and the bar shows the last answer in between.
pub const UTILITY_INTERVAL: Duration = Duration::from_secs(30);

/// Slice the poll waits in, so a dropped feed is noticed promptly.
const STOP_CHECK: Duration = Duration::from_millis(250);

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

/// Runs a program and hands back what it wrote.
pub trait Runner: Send + 'static {
    /// Runs the program, or fails the way the operating system did.
    fn run(&self, program: &str, args: &[&str]) -> io::Result<String>;
}

/// Runner that starts the program as a child process.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessRunner;

impl Runner for ProcessRunner {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<String> {
        let output = Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()?;

        if !output.status.success() {
            return Err(io::Error::other(format!(
                "{program} exited with {}",
                output.status
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Latest answer of a vendor utility, refreshed on a thread of its own.
#[derive(Debug)]
pub struct Feed {
    vendor: GpuVendor,
    latest: Arc<Mutex<Option<UtilityMetrics>>>,
    stop:   Arc<AtomicBool>
}

impl Feed {
    /// Starts polling a utility.
    ///
    /// `gate` decides whether the device may be queried at all, which
    /// is how a sleeping card is left alone instead of
    /// being woken up for a reading.
    pub fn spawn<R, G>(
        utility: &'static Utility,
        runner: R,
        gate: G,
        period: Duration
    ) -> Self
    where
        R: Runner,
        G: Fn() -> bool + Send + 'static
    {
        let latest = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));

        thread::spawn({
            let latest = Arc::clone(&latest);
            let stop = Arc::clone(&stop);

            move || {
                let mut reported = false;

                while !stop.load(Ordering::Relaxed) {
                    let sample = poll(utility, &runner, &gate, &mut reported);
                    store(&latest, sample);

                    let mut waited = Duration::ZERO;
                    while waited < period && !stop.load(Ordering::Relaxed) {
                        thread::sleep(STOP_CHECK.min(period.checked_sub(waited).unwrap()));
                        waited += STOP_CHECK;
                    }
                }
            }
        });

        Self {
            vendor: utility.vendor,
            latest,
            stop
        }
    }

    /// Vendor the feed reports on.
    #[must_use]
    pub const fn vendor(&self) -> GpuVendor {
        self.vendor
    }

    /// Latest metrics, or [`None`] while the device reports none.
    #[must_use]
    pub fn latest(&self) -> Option<UtilityMetrics> {
        self.latest.lock().ok().and_then(|latest| *latest)
    }
}

impl Drop for Feed {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn store(latest: &Mutex<Option<UtilityMetrics>>, sample: Option<UtilityMetrics>) {
    if let Ok(mut slot) = latest.lock() {
        *slot = sample;
    }
}

/// One run of a utility.
///
/// A closed gate and a program that failed both mean the same to the
/// bar: no reading right now. The failure is written to the log
/// once per feed, because a device that is simply absent must
/// not fill the journal every refresh.
fn poll<R, G>(
    utility: &Utility,
    runner: &R,
    gate: &G,
    reported: &mut bool
) -> Option<UtilityMetrics>
where
    R: Runner,
    G: Fn() -> bool
{
    if !gate() {
        return None;
    }

    match runner.run(utility.program, utility.args) {
        Ok(output) => {
            *reported = false;

            (utility.parse)(&output).filter(|metrics| !metrics.is_empty())
        }
        Err(error) => {
            if !*reported {
                debug!("{} reported no metrics: {error}", utility.program);
                *reported = true;
            }

            None
        }
    }
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
    use std::sync::atomic::AtomicUsize;

    use super::*;

    struct Scripted {
        output: &'static str,
        runs:   Arc<AtomicUsize>
    }

    impl Runner for Scripted {
        fn run(&self, _: &str, _: &[&str]) -> io::Result<String> {
            self.runs.fetch_add(1, Ordering::Relaxed);

            Ok(self.output.to_owned())
        }
    }

    struct Failing;

    impl Runner for Failing {
        fn run(&self, _: &str, _: &[&str]) -> io::Result<String> {
            Err(io::Error::new(io::ErrorKind::NotFound, "no such program"))
        }
    }

    fn nvidia() -> &'static Utility {
        for_vendor(GpuVendor::Nvidia).expect("a utility for the vendor")
    }

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
        let metrics =
            parse_query_row("66, 15, 1234, 8192\n71, 40, 512, 8192\n").expect("metrics");

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

    #[test]
    fn a_closed_gate_leaves_the_device_alone() {
        let runs = Arc::new(AtomicUsize::new(0));
        let runner = Scripted {
            output: "66, 15, 1234, 8192",
            runs:   Arc::clone(&runs)
        };
        let mut reported = false;

        assert_eq!(poll(nvidia(), &runner, &|| false, &mut reported), None);
        assert_eq!(runs.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn an_open_gate_reads_the_device() {
        let runs = Arc::new(AtomicUsize::new(0));
        let runner = Scripted {
            output: "66, 15, 1234, 8192",
            runs:   Arc::clone(&runs)
        };
        let mut reported = false;

        let metrics = poll(nvidia(), &runner, &|| true, &mut reported).expect("metrics");

        assert_eq!(metrics.temperature, Some(66));
        assert_eq!(runs.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_missing_program_is_reported_once_and_yields_nothing() {
        let mut reported = false;

        assert_eq!(poll(nvidia(), &Failing, &|| true, &mut reported), None);
        assert!(reported, "the first failure is written to the log");

        assert_eq!(poll(nvidia(), &Failing, &|| true, &mut reported), None);
        assert!(reported, "the next failures stay quiet");
    }

    #[test]
    fn a_feed_publishes_what_the_utility_answered() {
        let runs = Arc::new(AtomicUsize::new(0));
        let feed = Feed::spawn(
            nvidia(),
            Scripted {
                output: "66, 15, 1234, 8192",
                runs:   Arc::clone(&runs)
            },
            || true,
            Duration::from_millis(20)
        );

        let mut metrics = None;
        for _ in 0..200 {
            metrics = feed.latest();
            if metrics.is_some() {
                break;
            }

            thread::sleep(Duration::from_millis(5));
        }

        assert_eq!(feed.vendor(), GpuVendor::Nvidia);
        assert_eq!(metrics.expect("metrics").temperature, Some(66));
    }
}
