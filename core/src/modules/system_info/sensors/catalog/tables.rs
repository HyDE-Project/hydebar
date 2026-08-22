//! The lists a chip or an input is looked up in, and the rank that comes out.

use super::{
    GpuVendor,
    naming::{is_unnamed_input, normalise, normalise_chip}
};

/// Chip families whose readings stand for the processor, best tier
/// first.
///
/// The first tier is a driver bound to the processor's own thermal
/// registers, so it reports the die itself. The second is the
/// kernel package thermal driver, which reports the same die
/// through a coarser interface. The third is the board thermal
/// zone, which only tracks the processor on machines that
/// expose nothing better, and which a virtual machine may be the sole
/// source of.
pub(super) const CPU_CHIP_TIERS: [&[&str]; 3] = [
    &[
        "k10temp",
        "zenpower",
        "zenpower3",
        "coretemp",
        "k8temp",
        "cpu thermal",
        "cpu0 thermal",
        "soc thermal",
        "scpi sensors"
    ],
    &["x86 pkg temp"],
    &["acpitz"]
];

/// Chip families that report a graphics processor, with the vendor
/// behind them.
///
/// `nvidia` is the chip the vendor kernel module registers once it grew
/// monitoring support; `nouveau` is the in-tree driver for the same
/// cards. `i915` and `xe` are the two Intel drivers, and `gpu
/// thermal` is the zone name the ARM boards give the block
/// inside their system on a chip.
pub(super) const GPU_CHIPS: [(&str, GpuVendor); 8] = [
    ("amdgpu", GpuVendor::Amd),
    ("radeon", GpuVendor::Amd),
    ("nvidia", GpuVendor::Nvidia),
    ("nouveau", GpuVendor::Nvidia),
    ("i915", GpuVendor::Intel),
    ("xe", GpuVendor::Intel),
    ("gpu thermal", GpuVendor::SystemOnChip),
    ("mali", GpuVendor::SystemOnChip)
];

/// Processor inputs standing for the whole package, best first.
///
/// `tdie` is the die reading itself. `tctl` is the same die shifted by
/// the offset the cooling loop is tuned against, so it only
/// stands in when the driver publishes no die reading. `package
/// id` and `physical id` are the two spellings the kernel has
/// used for the package reading, and `package` and `cpu` cover
/// the drivers that name it plainly.
const CPU_PACKAGE_INPUTS: [&str; 6] = [
    "tdie",
    "tctl",
    "package id",
    "physical id",
    "package",
    "cpu"
];

/// Processor inputs covering a single core or a single die.
///
/// A core runs hotter than the package it sits in and says nothing
/// about the rest of the chip, so one is read only when the
/// driver publishes no package reading at all.
const CPU_CORE_INPUTS: [&str; 3] = ["core", "tccd", "die"];

/// Graphics inputs, best first.
///
/// `junction` and `hotspot` are the two names for the hottest spot on
/// the die, which is the reading the vendor tools show. `die`,
/// `gpu` and `core` are the plain die readings of the remaining
/// drivers. `edge` is measured at the rim of the package and
/// lags the die, so it stands in only when no die reading
/// exists. `mem` and `vram` measure the memory next to the die rather
/// than the die, and come last.
const GPU_INPUTS: [&str; 8] = [
    "junction", "hotspot", "die", "gpu", "core", "edge", "mem", "vram"
];

/// Rank of a per-core reading, worse than any package reading.
pub const CORE_INPUT_RANK: u8 = 100;

/// Rank of a reading whose name says nothing about what it measures.
///
/// A driver that labels nothing still reports something useful when it
/// is the only input of a chip the tables recognise, which is
/// how the vendor kernel module publishes the graphics
/// temperature.
pub const UNNAMED_INPUT_RANK: u8 = 200;

/// Folds a driver-supplied name into the spelling the tables carry.

/// Rank of a chip as a stand-in for the processor, lower being better.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the tier table holds far fewer than 256 entries"
)]
pub fn cpu_chip_rank(chip: &str) -> Option<u8> {
    let folded = normalise_chip(chip);

    CPU_CHIP_TIERS
        .iter()
        .position(|tier| tier.contains(&folded.as_str()))
        .map(|tier| tier as u8)
}

/// Vendor behind a chip that reports a graphics processor.
#[must_use]
pub fn gpu_vendor(chip: &str) -> Option<GpuVendor> {
    let folded = normalise_chip(chip);

    GPU_CHIPS
        .iter()
        .find(|(name, _)| *name == folded)
        .map(|(_, vendor)| *vendor)
}

/// Vendor behind a PCI vendor identifier, as sysfs spells it.
#[must_use]
pub fn gpu_vendor_from_pci(id: &str) -> Option<GpuVendor> {
    match id.trim().trim_start_matches("0x") {
        "1002" | "1022" => Some(GpuVendor::Amd),
        "8086" => Some(GpuVendor::Intel),
        "10de" => Some(GpuVendor::Nvidia),
        _ => None
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the rank tables hold far fewer than 256 entries"
)]
fn table_rank(table: &[&str], input: &str) -> Option<u8> {
    let folded = normalise(input);

    table
        .iter()
        .position(|candidate| folded.starts_with(candidate))
        .map(|position| position as u8)
}

/// Rank of an input as the processor reading of its chip, lower being
/// better.
#[must_use]
pub fn cpu_input_rank(input: &str) -> u8 {
    if is_unnamed_input(input) {
        return UNNAMED_INPUT_RANK;
    }

    table_rank(&CPU_PACKAGE_INPUTS, input)
        .or_else(|| table_rank(&CPU_CORE_INPUTS, input).map(|_| CORE_INPUT_RANK))
        .unwrap_or(UNNAMED_INPUT_RANK)
}

/// Rank of an input as the graphics reading of its chip, lower being
/// better.
#[must_use]
pub fn gpu_input_rank(input: &str) -> u8 {
    if is_unnamed_input(input) {
        return UNNAMED_INPUT_RANK;
    }

    table_rank(&GPU_INPUTS, input).unwrap_or(UNNAMED_INPUT_RANK)
}
