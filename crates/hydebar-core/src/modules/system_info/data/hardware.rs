//! What the machine is: names, ratings and firmware revisions.
//!
//! The identity of the hardware does not move between samples, so it
//! is read once and stamped onto every sample; only the frequency,
//! the governor and the page cache are re-read, because those are
//! the machine living, not the machine being.

use std::{fs, sync::LazyLock};

/// Step the current frequency is damped to, in MHz.
///
/// An idle processor wanders by tens of MHz between samples; a
/// window repainting on that wander would keep the whole bar warm
/// for a number nobody can read that fast.
const FREQUENCY_STEP_MHZ: u32 = 50;

/// Step the page cache reading is damped to.
const CACHE_STEP_BYTES: u64 = 64 * 1024 * 1024;

/// The unchanging part, read once per process.
pub struct Identity {
    pub model:     Option<String>,
    pub cores:     Option<u32>,
    pub max_mhz:   Option<u32>,
    pub microcode: Option<String>,
    pub kernel:    Option<String>,
    pub swap:      Option<String>
}

static IDENTITY: LazyLock<Identity> = LazyLock::new(|| {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let (model, cores, microcode) = parse_cpuinfo(&cpuinfo);

    Identity {
        model,
        cores,
        max_mhz: read_khz("/sys/devices/system/cpu/cpufreq/policy0/cpuinfo_max_freq"),
        microcode,
        kernel: fs::read_to_string("/proc/sys/kernel/osrelease")
            .ok()
            .map(|kernel| kernel.trim().to_owned()),
        swap: swap_backend()
    }
});

/// The unchanging part of the machine.
pub fn identity() -> &'static Identity {
    &IDENTITY
}

/// Fastest core right now, damped, in MHz.
pub fn current_mhz() -> Option<u32> {
    let policies = fs::read_dir("/sys/devices/system/cpu/cpufreq").ok()?;

    policies
        .flatten()
        .filter_map(|policy| {
            read_khz(policy.path().join("scaling_cur_freq").to_str()?)
        })
        .max()
        .map(|mhz| mhz / FREQUENCY_STEP_MHZ * FREQUENCY_STEP_MHZ)
}

/// Governor steering the first policy, which steers them all on any
/// machine that has not been hand-tuned per core.
pub fn governor() -> Option<String> {
    fs::read_to_string("/sys/devices/system/cpu/cpufreq/policy0/scaling_governor")
        .ok()
        .map(|governor| governor.trim().to_owned())
        .filter(|governor| !governor.is_empty())
}

/// Page cache and reclaimable slab, damped, in bytes.
pub fn cached_bytes() -> u64 {
    let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();

    parse_cached_kib(&meminfo) * 1024 / CACHE_STEP_BYTES * CACHE_STEP_BYTES
}

/// A frequency file stated in kHz, read as MHz.
fn read_khz(path: &str) -> Option<u32> {
    fs::read_to_string(path)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()
        .map(|khz| khz / 1000)
}

/// Model, physical cores and microcode out of `/proc/cpuinfo`.
pub(super) fn parse_cpuinfo(
    cpuinfo: &str
) -> (Option<String>, Option<u32>, Option<String>) {
    let field = |name: &str| {
        cpuinfo.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;

            (key.trim() == name).then(|| value.trim().to_owned())
        })
    };

    (
        field("model name"),
        field("cpu cores").and_then(|cores| cores.parse().ok()),
        field("microcode")
    )
}

/// Cached and reclaimable slab out of `/proc/meminfo`, in KiB.
pub(super) fn parse_cached_kib(meminfo: &str) -> u64 {
    let field = |name: &str| {
        meminfo.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;

            (key.trim() == name)
                .then(|| value.trim().trim_end_matches(" kB").trim().parse().ok())?
        })
    };

    field("Cached").unwrap_or(0u64) + field("SReclaimable").unwrap_or(0)
}

/// The device swap lives on, with its compression when it is zram.
fn swap_backend() -> Option<String> {
    let swaps = fs::read_to_string("/proc/swaps").ok()?;
    let device = swaps.lines().nth(1)?.split_whitespace().next()?.to_owned();
    let name = device.rsplit('/').next().unwrap_or(&device);

    let algorithm = name.starts_with("zram").then(|| {
        fs::read_to_string(format!("/sys/block/{name}/comp_algorithm"))
            .ok()
            .and_then(|algorithms| selected_algorithm(&algorithms))
    });

    Some(match algorithm.flatten() {
        Some(algorithm) => format!("{device} · zram ({algorithm})"),
        None => device
    })
}

/// The bracketed choice out of a kernel choice list like
/// `lzo lz4 [zstd] deflate`.
pub(super) fn selected_algorithm(algorithms: &str) -> Option<String> {
    let start = algorithms.find('[')?;
    let end = algorithms[start..].find(']')? + start;

    Some(algorithms[start + 1..end].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CPUINFO: &str = "processor\t: 0\n\
        model name\t: AMD RYZEN AI MAX+ 395 w/ Radeon 8060S\n\
        microcode\t: 0xb700037\n\
        cpu cores\t: 16\n";

    #[test]
    fn the_model_the_cores_and_the_microcode_are_read() {
        let (model, cores, microcode) = parse_cpuinfo(CPUINFO);

        assert_eq!(
            model.as_deref(),
            Some("AMD RYZEN AI MAX+ 395 w/ Radeon 8060S")
        );
        assert_eq!(cores, Some(16));
        assert_eq!(microcode.as_deref(), Some("0xb700037"));
    }

    #[test]
    fn a_machine_stating_nothing_reads_as_nothing() {
        assert_eq!(parse_cpuinfo(""), (None, None, None));
    }

    #[test]
    fn the_page_cache_counts_the_reclaimable_slab() {
        let meminfo = "Buffers:              16 kB\n\
            Cached:         19290036 kB\n\
            SReclaimable:     182832 kB\n";

        assert_eq!(parse_cached_kib(meminfo), 19_290_036 + 182_832);
    }

    #[test]
    fn the_selected_compression_is_the_bracketed_one() {
        assert_eq!(
            selected_algorithm("lzo-rle lzo lz4 lz4hc [zstd] deflate 842").as_deref(),
            Some("zstd")
        );
        assert_eq!(selected_algorithm("zstd"), None);
    }
}
