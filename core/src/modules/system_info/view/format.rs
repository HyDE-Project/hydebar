//! The one spelling of every readout: sizes, rates, names and glyphs.

use super::super::sensors::{GpuPlacement, GpuReadings};
use crate::{components::icons::Icons, config::MemoryFormat};

/// Builds the text of an indicator out of an optional prefix, a value and
/// its unit.
pub(super) fn indicator_label(
    prefix: Option<&str>,
    value: impl std::fmt::Display,
    unit: &str
) -> String {
    prefix.map_or_else(
        || format!("{value}{unit}"),
        |prefix| format!("{prefix} {value}{unit}")
    )
}

/// Amount of bytes rendered as gibibytes with a single decimal.
///
/// The divisor is binary, so the unit next to the number has to be the
/// binary one: eight gibibytes shown as `8.0GB` overstated every
/// readout by seven percent against the unit it named.
#[expect(
    clippy::cast_precision_loss,
    reason = "byte totals are shown with one decimal; f64 keeps far more precision than the display"
)]
pub fn gigabytes(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// A pool stated as the amount in use against the amount there is.
pub fn used_of_total(used: u64, total: u64) -> String {
    format!("{} / {} GiB", gigabytes(used), gigabytes(total))
}

/// Renders a memory readout in the format the active index selects.
pub(super) fn memory_label(
    format: MemoryFormat,
    prefix: Option<&str>,
    usage: u32,
    used: u64
) -> String {
    match format {
        MemoryFormat::Percentage => indicator_label(prefix, usage, "%"),
        MemoryFormat::Bytes => indicator_label(prefix, gigabytes(used), "GiB")
    }
}

/// Name of a graphics device as the menu spells it out.
///
/// The placement is spelled out rather than abbreviated, so a machine with
/// switchable graphics says which of its two devices the bar is watching.
pub fn gpu_title(gpu: &GpuReadings) -> String {
    let placement = match gpu.placement {
        GpuPlacement::Integrated => "Integrated graphics",
        GpuPlacement::Discrete | GpuPlacement::Unknown => "Graphics"
    };

    gpu.source.as_deref().map_or_else(
        || format!("{placement} ({})", gpu.name),
        |source| format!("{placement} ({source})")
    )
}

/// Glyph a graphics device wears on the bar.
///
/// An integrated device gets a glyph of its own instead of a text tag beside
/// the number, so every readout on the bar is one icon and one value.
pub(super) const fn gpu_icon(gpu: &GpuReadings) -> Icons {
    match gpu.placement {
        GpuPlacement::Integrated => Icons::IntegratedGpu,
        GpuPlacement::Discrete | GpuPlacement::Unknown => Icons::Gpu
    }
}

/// A transfer rate, handed in as kilobytes per second, spelled out.
///
/// Above a thousand the rate reads in megabytes with one decimal: the
/// integer division it replaced showed `1 MB/s` for anything up to
/// `1999 KB/s`, understating a rate by up to half.
pub fn format_speed(speed: u32) -> String {
    if speed >= 1000 {
        format!("{:.1} MB/s", f64::from(speed) / 1000.0)
    } else {
        format!("{speed} KB/s")
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn format_speed_converts_large_values_to_megabytes() {
        assert_eq!(format_speed(2048), "2.0 MB/s");
    }

    #[test]
    fn format_speed_keeps_the_fraction_a_truncation_used_to_drop() {
        assert_eq!(format_speed(1999), "2.0 MB/s");
        assert_eq!(format_speed(1500), "1.5 MB/s");
        assert_eq!(format_speed(999), "999 KB/s");
    }

    #[test]
    fn the_memory_readout_follows_the_active_format() {
        assert_eq!(
            memory_label(MemoryFormat::Percentage, None, 50, 8 * 1024 * 1024 * 1024),
            "50%"
        );
        assert_eq!(
            memory_label(MemoryFormat::Bytes, None, 50, 8 * 1024 * 1024 * 1024),
            "8.0GiB"
        );
    }

    #[test]
    fn a_prefixed_memory_readout_keeps_its_prefix_in_both_formats() {
        assert_eq!(
            memory_label(
                MemoryFormat::Percentage,
                Some("swap"),
                10,
                1024 * 1024 * 1024
            ),
            "swap 10%"
        );
        assert_eq!(
            memory_label(MemoryFormat::Bytes, Some("swap"), 10, 1024 * 1024 * 1024),
            "swap 1.0GiB"
        );
    }

    #[test]
    fn gigabytes_round_to_a_single_decimal() {
        assert_eq!(gigabytes(0), "0.0");
        assert_eq!(gigabytes(1024 * 1024 * 1024 * 3 / 2), "1.5");
    }

    #[test]
    fn a_pool_reads_as_used_against_total() {
        assert_eq!(
            used_of_total(8 * 1024 * 1024 * 1024, 16 * 1024 * 1024 * 1024),
            "8.0 / 16.0 GiB"
        );
    }
}
