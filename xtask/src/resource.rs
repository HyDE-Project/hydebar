//! `cargo xtask resource-check [PID]`: the resource budget of a running bar.
//!
//! The bar earns its place on a desktop by costing almost nothing while it
//! stands there. This tool turns that claim into a number: it attaches to a
//! running `hydebar`, lets it settle, then reads the kernel's own accounting
//! from `/proc` — resident and anonymous memory, thread count, and the CPU it
//! burns while idle — and checks each against a budget. Exit status is `1`
//! when any budget is exceeded, so the same command gates a release.
//!
//! It measures a process that is already running rather than spawning one:
//! the bar only lives under a Wayland compositor, and a harness that started
//! its own compositor would be testing the compositor's CI, not the bar's.
//! Point it at the bar of a live session (or pass a PID) to take a baseline.
//!
//! Budgets are read from the environment so a baseline can be tightened
//! without a code change: `RSS_MAX_KIB`, `ANON_MAX_KIB`, `THREADS_MAX`,
//! `CPU_MAX_PCT`, `SETTLE_SECS`, `CPU_SECS`. The defaults are the goals the
//! bar is meant to hit, not a measured floor — set them from a real baseline
//! before wiring this into a release gate.

use std::{
    collections::BTreeMap,
    thread::sleep,
    time::{Duration, Instant}
};

/// Resident memory budget, in kibibytes.
const RSS_MAX_KIB: u64 = 20 * 1024;
/// Anonymous (heap) memory budget, in kibibytes.
const ANON_MAX_KIB: u64 = 16 * 1024;
/// Thread budget.
const THREADS_MAX: usize = 10;
/// Idle CPU budget, in percent of one core.
const CPU_MAX_PCT: f64 = 0.1;
/// Seconds to let a freshly started bar settle before measuring.
const SETTLE_SECS: u64 = 5;
/// Seconds over which idle CPU is averaged.
const CPU_SECS: u64 = 30;
/// The process name the bar runs under.
const BAR_NAME: &str = "hydebar";
/// Fallback clock ticks per second when the system does not say.
const FALLBACK_HZ: u64 = 100;
/// Percent, for turning a tick ratio into a percentage.
const PERCENT: f64 = 100.0;

/// One budget line: a measured value against the ceiling it must respect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budget {
    /// Human name of the metric.
    pub name:  &'static str,
    /// The measured value.
    pub value: f64,
    /// The ceiling the value must not exceed.
    pub max:   f64
}

impl Budget {
    /// Whether the measurement respects its budget.
    #[must_use]
    pub const fn holds(&self) -> bool {
        self.value <= self.max
    }

    /// The report line for this budget.
    #[must_use]
    pub fn report(&self) -> String {
        if self.holds() {
            format!(
                "ok   {} {:.3} (budget {:.3})",
                self.name, self.value, self.max
            )
        } else {
            format!("FAIL {} {:.3} > {:.3}", self.name, self.value, self.max)
        }
    }
}

/// Reads a `Key:   N kB` field out of an `smaps_rollup` dump.
#[must_use]
pub fn rollup_kib(rollup: &str, key: &str) -> Option<u64> {
    let wanted = format!("{key}:");

    rollup.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        (fields.next()? == wanted)
            .then(|| fields.next()?.parse().ok())
            .flatten()
    })
}

/// User plus system CPU time from a `/proc/<pid>/stat` line, in clock ticks.
///
/// The command name is wrapped in parentheses and may itself contain spaces,
/// so the count restarts after the last `)`.
#[must_use]
pub fn stat_cpu_ticks(stat: &str) -> Option<u64> {
    let after_name = &stat[stat.rfind(')')? + 1..];
    let mut fields = after_name.split_whitespace();
    // Fields after the name start at `state` (stat field 3); `utime` is field
    // 14 and `stime` field 15, so they sit at index 11 and 12 here.
    let utime = fields.nth(11)?.parse::<u64>().ok()?;
    let stime = fields.next()?.parse::<u64>().ok()?;

    Some(utime + stime)
}

/// The share of one core a process used, given the ticks it spent over a span.
#[expect(
    clippy::cast_precision_loss,
    reason = "tick counts and a refresh divisor are far below the 2^53 an f64 holds exactly"
)]
#[must_use]
pub fn idle_cpu_percent(ticks: u64, hz: u64, seconds: u64) -> f64 {
    let seconds = u64::max(seconds, 1) as f64;
    let consumed = ticks as f64 / f64::max(hz as f64, 1.0);

    consumed * PERCENT / seconds
}

/// Strips trailing digits so thread names collapse into families.
#[must_use]
pub fn strip_trailing_digits(name: &str) -> &str {
    name.trim_end_matches(|c: char| c.is_ascii_digit())
}

/// Groups thread names into `(count, family)` pairs, busiest family first.
#[must_use]
pub fn group_thread_names<I, S>(comms: I) -> Vec<(usize, String)>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>
{
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();

    for comm in comms {
        let family = strip_trailing_digits(comm.as_ref().trim_end());
        *counts.entry(family.to_owned()).or_default() += 1;
    }

    let mut grouped: Vec<(usize, String)> = counts
        .into_iter()
        .map(|(name, count)| (count, name))
        .collect();
    grouped.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    grouped
}

/// Reads an environment budget, falling back to the default when unset.
fn env_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

/// Clock ticks per second, the divisor that turns stat ticks into seconds.
#[expect(
    unsafe_code,
    reason = "sysconf(_SC_CLK_TCK) reads a cached system value, allocates nothing and takes no locks"
)]
fn clock_ticks_per_second() -> u64 {
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };

    u64::try_from(hz).unwrap_or(FALLBACK_HZ).max(1)
}

/// The process ids whose command name is exactly the bar's.
#[must_use]
pub fn find_bar_pids() -> Vec<i32> {
    let mut pids = Vec::new();

    let Ok(entries) = std::fs::read_dir("/proc") else {
        return pids;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid): Option<i32> = name.to_str().and_then(|s| s.parse().ok()) else {
            continue;
        };

        if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm"))
            && comm.trim_end() == BAR_NAME
        {
            pids.push(pid);
        }
    }

    pids
}

/// Resident memory of `pid`, in kibibytes.
fn resident_kib(pid: i32) -> Option<u64> {
    let rollup = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")).ok()?;
    rollup_kib(&rollup, "Rss")
}

/// Anonymous memory of `pid`, in kibibytes.
fn anonymous_kib(pid: i32) -> Option<u64> {
    let rollup = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")).ok()?;
    rollup_kib(&rollup, "Anonymous")
}

/// CPU time of `pid`, in clock ticks.
fn cpu_ticks(pid: i32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat_cpu_ticks(&stat)
}

/// The command names of every thread of `pid`.
fn thread_comms(pid: i32) -> Vec<String> {
    std::fs::read_dir(format!("/proc/{pid}/task")).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries
                .flatten()
                .filter_map(|entry| {
                    std::fs::read_to_string(entry.path().join("comm"))
                        .ok()
                        .map(|comm| comm.trim_end().to_owned())
                })
                .collect()
        }
    )
}

/// Measures `pid` against the budgets and prints the report.
///
/// Returns whether every budget held.
#[expect(
    clippy::cast_precision_loss,
    reason = "kibibyte and thread counts are far below the 2^53 an f64 holds exactly"
)]
#[must_use]
pub fn measure(pid: i32) -> bool {
    let rss_max = env_or("RSS_MAX_KIB", RSS_MAX_KIB);
    let anon_max = env_or("ANON_MAX_KIB", ANON_MAX_KIB);
    let threads_max = env_or("THREADS_MAX", THREADS_MAX);
    let cpu_max = env_or("CPU_MAX_PCT", CPU_MAX_PCT);
    let settle = env_or("SETTLE_SECS", SETTLE_SECS);
    let cpu_secs = env_or("CPU_SECS", CPU_SECS);

    if settle > 0 {
        sleep(Duration::from_secs(settle));
    }

    let hz = clock_ticks_per_second();
    let start = cpu_ticks(pid);
    let clock = Instant::now();
    sleep(Duration::from_secs(cpu_secs));
    let elapsed = cpu_secs.max(clock.elapsed().as_secs());
    let cpu = match (start, cpu_ticks(pid)) {
        (Some(first), Some(last)) => idle_cpu_percent(last.saturating_sub(first), hz, elapsed),
        _ => f64::INFINITY
    };

    let budgets = [
        Budget {
            name:  "rss_kib",
            value: resident_kib(pid).unwrap_or(u64::MAX) as f64,
            max:   rss_max as f64
        },
        Budget {
            name:  "anon_kib",
            value: anonymous_kib(pid).unwrap_or(u64::MAX) as f64,
            max:   anon_max as f64
        },
        Budget {
            name:  "threads",
            value: thread_comms(pid).len() as f64,
            max:   threads_max as f64
        },
        Budget {
            name:  "idle_cpu_pct",
            value: cpu,
            max:   cpu_max
        }
    ];

    let held = budgets.iter().all(Budget::holds);

    for budget in &budgets {
        println!("{}", budget.report());
    }

    if !held {
        println!("threads by family:");
        for (count, family) in group_thread_names(thread_comms(pid)) {
            println!("{count:>5} {family}");
        }
    }

    held
}

/// Entry point for `cargo xtask resource-check [PID]`.
///
/// Resolves the bar to measure — an explicit PID or the single running bar —
/// then reports against the budgets. Returns the process exit code.
pub fn resource_check(args: &[String]) -> Result<bool, String> {
    let pid = match args.first() {
        Some(text) => text
            .parse::<i32>()
            .map_err(|_| format!("'{text}' is not a pid"))?,
        None => match find_bar_pids().as_slice() {
            [only] => *only,
            [] => return Err("no running hydebar found; pass a PID".to_owned()),
            many => {
                return Err(format!(
                    "{} bars are running ({}); pass the one to measure",
                    many.len(),
                    many.iter()
                        .map(i32::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    };

    if std::fs::read_to_string(format!("/proc/{pid}/stat")).is_err() {
        return Err(format!("no process {pid}"));
    }

    println!("measuring hydebar pid {pid}");

    Ok(measure(pid))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn reads_resident_and_anonymous_from_the_rollup() {
        let rollup = "00400000-7ffd ---p 00000000 00:00 0 [rollup]\n\
                      Rss:              41200 kB\n\
                      Pss:              30000 kB\n\
                      Anonymous:        19000 kB\n";

        assert_eq!(rollup_kib(rollup, "Rss"), Some(41200));
        assert_eq!(rollup_kib(rollup, "Anonymous"), Some(19000));
    }

    #[test]
    fn a_missing_rollup_field_reads_as_none() {
        assert_eq!(rollup_kib("Rss:  1 kB\n", "Swap"), None);
    }

    #[test]
    fn a_malformed_rollup_number_reads_as_none() {
        assert_eq!(rollup_kib("Rss:  many kB\n", "Rss"), None);
    }

    #[test]
    fn cpu_ticks_sum_user_and_system_time() {
        // After the name: state, ppid, pgrp, session, tty, tpgid, flags,
        // minflt, cminflt, majflt, cmajflt, then utime and stime.
        let stat = "42 (hydebar) S 1 1 1 1 1 1 1 1 1 1 30 4 0 0 20 0 1 0 999";

        assert_eq!(stat_cpu_ticks(stat), Some(34));
    }

    #[test]
    fn cpu_ticks_survive_a_command_name_with_spaces_and_parens() {
        let stat = "42 (weird (name) bar) S 1 1 1 1 1 1 1 1 1 1 30 4 0 0 20 0 1 0 999";

        assert_eq!(stat_cpu_ticks(stat), Some(34));
    }

    #[test]
    fn cpu_ticks_of_a_malformed_stat_reads_as_none() {
        assert_eq!(stat_cpu_ticks("no parens here"), None);
    }

    #[test]
    fn idle_cpu_is_a_share_of_one_core() {
        // 100 ticks at 100 Hz over 10 seconds is one full core for one second:
        // ten percent of the window.
        assert!((idle_cpu_percent(100, 100, 10) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn idle_cpu_never_divides_by_zero() {
        assert_eq!(idle_cpu_percent(0, 0, 0), 0.0);
    }

    #[test]
    fn thread_families_collapse_trailing_digits_and_sort_by_size() {
        let comms = [
            "tokio-worker1",
            "tokio-worker2",
            "tokio-worker3",
            "main",
            "wgpu0"
        ];
        let grouped = group_thread_names(comms);

        assert_eq!(
            grouped.first().map(|(count, name)| (*count, name.as_str())),
            Some((3, "tokio-worker"))
        );
        assert!(grouped.iter().any(|(_, name)| name == "wgpu"));
    }

    #[test]
    fn a_budget_holds_at_its_ceiling_and_fails_above_it() {
        let at = Budget {
            name:  "threads",
            value: 10.0,
            max:   10.0
        };
        let over = Budget {
            name:  "threads",
            value: 11.0,
            max:   10.0
        };

        assert!(at.holds());
        assert!(!over.holds());
        assert!(over.report().starts_with("FAIL"));
        assert!(at.report().starts_with("ok"));
    }

    #[test]
    fn a_budget_reports_its_measurement() {
        let budget = Budget {
            name:  "rss_kib",
            value: 100.0,
            max:   200.0
        };

        assert!(budget.report().contains("rss_kib"));
        assert!(budget.report().contains("budget"));
    }
}
