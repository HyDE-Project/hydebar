//! Which monitoring chip stands for which piece of hardware, and which
//! of its inputs to trust.
//!
//! Every decision here is a table lookup: adding a chip family or a
//! vendor is an entry in a list, never a condition spread over
//! the module. Nothing in this file reads the machine, so the
//! whole selection can be exercised against a written-down list
//! of chips and input labels.

/// Vendor behind a graphics processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuVendor {
    Amd,
    Intel,
    Nvidia,
    /// A graphics block integrated into a system on a chip, such as the
    /// ones the ARM boards expose through their thermal
    /// zones.
    SystemOnChip
}

impl GpuVendor {
    /// Name the vendor is addressed by in the configuration and in the
    /// menu.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Amd => "amd",
            Self::Intel => "intel",
            Self::Nvidia => "nvidia",
            Self::SystemOnChip => "soc"
        }
    }
}

/// Where a graphics processor sits relative to the processor package.
///
/// The ordering is the preference order: a card outranks a part whose
/// placement cannot be established, which in turn outranks the block
/// built into the processor, so a laptop with switchable
/// graphics reports the card whenever the card is awake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GpuPlacement {
    Discrete,
    Unknown,
    Integrated
}

impl GpuPlacement {
    /// Short tag the bar puts in front of the reading.
    ///
    /// Only the integrated block is tagged: a user looking at a machine
    /// with a card must never mistake the block inside the
    /// processor for it, while a machine with a single card
    /// gains nothing from a prefix.
    #[must_use]
    pub const fn tag(self) -> Option<&'static str> {
        match self {
            Self::Integrated => Some("iGPU"),
            Self::Discrete | Self::Unknown => None
        }
    }
}

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
const CPU_CHIP_TIERS: [&[&str]; 3] = [
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
const GPU_CHIPS: [(&str, GpuVendor); 8] = [
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
///
/// Kernels differ in case, in whether they separate words with a space,
/// a dash or an underscore, and in the numeric suffix they
/// append to the second instance of a chip, so every spelling
/// folds onto one entry instead of the tables listing them all.
#[must_use]
pub fn normalise(value: &str) -> String {
    let mut folded = String::with_capacity(value.len());
    let mut pending_space = false;

    for character in value.chars() {
        if character.is_whitespace() || character == '_' || character == '-' {
            pending_space = !folded.is_empty();
            continue;
        }

        if pending_space {
            folded.push(' ');
            pending_space = false;
        }

        folded.extend(character.to_lowercase());
    }

    folded
}

/// Folds a chip name and drops the instance number the kernel appends.
///
/// A second chip of the same family arrives as `acpitz_1`, and a
/// machine with two processor packages arrives as `coretemp`
/// twice, so the number belongs to the instance rather than to
/// the family.
#[must_use]
pub fn normalise_chip(chip: &str) -> String {
    let folded = normalise(chip);

    match folded.rsplit_once(' ') {
        Some((base, suffix))
            if !base.is_empty()
                && !suffix.is_empty()
                && suffix.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base.to_owned()
        }
        _ => folded
    }
}

/// Reports whether an input label carries no information about what it
/// measures.
///
/// A driver that labels nothing leaves `tempN_label` absent, and the
/// kernel interface names the file itself `tempN`, so both
/// forms mean the same.
#[must_use]
pub fn is_unnamed_input(input: &str) -> bool {
    let folded = normalise(input);

    folded.is_empty()
        || folded
            .strip_prefix("temp")
            .is_some_and(|rest| rest.bytes().all(|b| b.is_ascii_digit()))
}

/// Rank of a chip as a stand-in for the processor, lower being better.
#[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_fold_across_spellings() {
        assert_eq!(normalise("Package id 0"), "package id 0");
        assert_eq!(normalise("cpu-thermal"), "cpu thermal");
        assert_eq!(normalise("CPU_THERMAL"), "cpu thermal");
        assert_eq!(normalise("  Tctl  "), "tctl");
    }

    #[test]
    fn a_chip_keeps_its_family_across_instances() {
        assert_eq!(normalise_chip("acpitz_0"), "acpitz");
        assert_eq!(normalise_chip("coretemp"), "coretemp");
        assert_eq!(normalise_chip("x86_pkg_temp"), "x86 pkg temp");
        assert_eq!(normalise_chip("mt7925_phy0"), "mt7925 phy0");
    }

    #[test]
    fn only_processor_chips_rank_as_the_processor() {
        assert_eq!(cpu_chip_rank("k10temp"), Some(0));
        assert_eq!(cpu_chip_rank("coretemp"), Some(0));
        assert_eq!(cpu_chip_rank("x86_pkg_temp"), Some(1));
        assert_eq!(cpu_chip_rank("acpitz_0"), Some(2));
        assert_eq!(cpu_chip_rank("amdgpu"), None);
        assert_eq!(cpu_chip_rank("nvme"), None);
        assert_eq!(cpu_chip_rank("r8169_0_c100:00"), None);
        assert_eq!(cpu_chip_rank("mt7925_phy0"), None);
    }

    #[test]
    fn only_graphics_chips_carry_a_vendor() {
        assert_eq!(gpu_vendor("amdgpu"), Some(GpuVendor::Amd));
        assert_eq!(gpu_vendor("nouveau"), Some(GpuVendor::Nvidia));
        assert_eq!(gpu_vendor("nvidia"), Some(GpuVendor::Nvidia));
        assert_eq!(gpu_vendor("i915"), Some(GpuVendor::Intel));
        assert_eq!(gpu_vendor("gpu-thermal"), Some(GpuVendor::SystemOnChip));
        assert_eq!(gpu_vendor("k10temp"), None);
        assert_eq!(gpu_vendor("nvme"), None);
    }

    #[test]
    fn the_die_reading_outranks_the_control_and_the_package() {
        assert!(cpu_input_rank("Tdie") < cpu_input_rank("Tctl"));
        assert!(cpu_input_rank("Tctl") < cpu_input_rank("Package id 0"));
        assert!(cpu_input_rank("Package id 0") < cpu_input_rank("Core 0"));
        assert!(cpu_input_rank("Core 0") < cpu_input_rank("temp1"));
        assert!(
            cpu_input_rank("Physical id 0") < cpu_input_rank("Core 0"),
            "both spellings of the package reading beat a core"
        );
    }

    #[test]
    fn the_graphics_die_outranks_the_edge_and_the_memory() {
        assert!(gpu_input_rank("junction") < gpu_input_rank("edge"));
        assert!(gpu_input_rank("edge") < gpu_input_rank("mem"));
        assert!(gpu_input_rank("mem") < gpu_input_rank("temp1"));
    }

    #[test]
    fn an_unlabelled_input_is_recognised_in_both_forms() {
        assert!(is_unnamed_input(""));
        assert!(is_unnamed_input("temp1"));
        assert!(!is_unnamed_input("edge"));
        assert!(!is_unnamed_input("Tctl"));
    }

    #[test]
    fn no_chip_stands_for_both_the_processor_and_the_graphics() {
        // One chip in both tables would put the same reading on the bar twice,
        // once as the processor and once as the graphics, so the two tables
        // have to stay disjoint however many families they grow.
        for tier in CPU_CHIP_TIERS {
            for chip in tier {
                assert_eq!(
                    gpu_vendor(chip),
                    None,
                    "{chip} stands for the processor already"
                );
            }
        }

        for (chip, _) in GPU_CHIPS {
            assert_eq!(
                cpu_chip_rank(chip),
                None,
                "{chip} stands for the graphics already"
            );
        }
    }

    #[test]
    fn pci_identifiers_resolve_to_vendors() {
        assert_eq!(gpu_vendor_from_pci("0x10de"), Some(GpuVendor::Nvidia));
        assert_eq!(gpu_vendor_from_pci("0x1002"), Some(GpuVendor::Amd));
        assert_eq!(gpu_vendor_from_pci("0x8086"), Some(GpuVendor::Intel));
        assert_eq!(gpu_vendor_from_pci("0x1234"), None);
    }
}
