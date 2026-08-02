//! The thread that keeps asking the vendor utility, and the latest
//! answer it holds for the panel.

use std::{
    io,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering}
    },
    thread,
    time::Duration
};

use log::debug;

use super::{GpuVendor, Utility, UtilityMetrics};

/// Slice the poll waits in, so a dropped feed is noticed promptly.
const STOP_CHECK: Duration = Duration::from_millis(250);

/// Runs a program and hands back what it wrote.
pub trait Runner: Send + 'static {
    /// Runs the program, or fails the way the operating system did.
    ///
    /// # Errors
    ///
    /// Returns an error when the program cannot be started or exits
    /// unsuccessfully.
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
    pub fn spawn<R, G>(utility: &'static Utility, runner: R, gate: G, period: Duration) -> Self
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
                        thread::sleep(STOP_CHECK.min(period.saturating_sub(waited)));
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::{super::for_vendor, *};

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
