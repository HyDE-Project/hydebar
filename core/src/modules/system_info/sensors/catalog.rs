//! Which monitoring chip stands for which piece of hardware, and which
//! of its inputs to trust.
//!
//! Every decision here is a table lookup: adding a chip family or a
//! vendor is an entry in a list, never a condition spread over
//! the module. Nothing in this file reads the machine, so the
//! whole selection can be exercised against a written-down list
//! of chips and input labels.
//!
//! Three rooms. What a chip may be is named here; [`naming`] folds the
//! spellings the kernel uses into one, and [`tables`] holds the lists a fold
//! is looked up in and the ranking that comes out.

mod naming;
mod tables;

pub use naming::{is_unnamed_input, normalise, normalise_chip};
pub use tables::{
    CORE_INPUT_RANK, UNNAMED_INPUT_RANK, cpu_chip_rank, cpu_input_rank, gpu_input_rank,
    gpu_vendor, gpu_vendor_from_pci
};
#[cfg(test)]
use tables::{CPU_CHIP_TIERS, GPU_CHIPS};

/// Vendor behind a graphics processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuVendor {
    /// A card from AMD.
    Amd,
    /// A card from Intel.
    Intel,
    /// A card from NVIDIA.
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
    /// A card of its own.
    Discrete,
    /// Neither could be told from what the machine reports.
    Unknown,
    /// Graphics built into the processor.
    Integrated
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
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
