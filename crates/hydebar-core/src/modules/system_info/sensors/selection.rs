//! Picking one reading per role out of everything the machine
//! publishes.
//!
//! The selection never touches the machine: it works on chips and input
//! labels, so the whole preference order is exercised against
//! written-down sensor sets rather than against whatever
//! hardware the test runs on.

use super::catalog::{
    self, GpuPlacement, GpuVendor, cpu_chip_rank, cpu_input_rank, gpu_input_rank,
    gpu_vendor
};

/// Inputs of a chip that are not temperatures but tell one part from
/// another.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ChipFacts {
    /// The chip publishes the northbridge voltage rail.
    ///
    /// The graphics driver exposes that rail only for a graphics block
    /// built into the processor, which is the one signal in
    /// the monitoring subsystem that separates it from a
    /// card.
    pub northbridge_voltage: bool,
    /// The chip publishes a fan reading.
    ///
    /// A graphics block built into the processor is cooled by the
    /// processor fan and owns none, so a fan of its own
    /// points at a card.
    pub fan:                 bool,
    /// The device sits directly on the root complex of its bus.
    ///
    /// Intel builds its graphics into the root complex and gives it the
    /// same address on every generation, while a card of
    /// theirs is reached through a bridge, so the address
    /// separates the two. [`None`] marks a device
    /// whose address says nothing, such as one on a system on a chip.
    pub on_root_complex:     Option<bool>
}

impl ChipFacts {
    /// Facts an address on the bus tells about a device.
    ///
    /// The address is the last element of the device path, spelled
    /// `<domain>:<bus>:<device>.<function>` the way the kernel writes
    /// it.
    #[must_use]
    pub fn from_address(address: Option<&str>) -> Self {
        Self {
            on_root_complex: address.and_then(root_complex_bus),
            ..Self::default()
        }
    }
}

/// Reports whether an address names a device on the root complex of its
/// bus.
fn root_complex_bus(address: &str) -> Option<bool> {
    let mut parts = address.split(':');
    let _domain = parts.next()?;
    let bus = parts.next()?;

    parts.next()?;

    Some(bus.bytes().all(|byte| byte == b'0'))
}

/// A monitoring chip as the selection sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChipView<'a> {
    /// Name the driver registered, such as `k10temp` or `amdgpu`.
    pub chip:   &'a str,
    /// Label of every temperature input, empty where the driver labels
    /// none.
    pub inputs: &'a [&'a str],
    pub facts:  ChipFacts
}

/// One chosen reading, addressed the way it was handed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    pub chip:  usize,
    pub input: usize
}

/// A chosen graphics reading together with what is known about the
/// device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuPick {
    pub chip:      usize,
    pub input:     usize,
    pub vendor:    GpuVendor,
    pub placement: GpuPlacement
}

/// Where a graphics chip sits, judged from what it publishes.
///
/// Cards driven by the vendor module and by its in-tree counterpart are
/// always add-in parts on the machines those drivers bind to.
/// For the remaining vendors the placement follows the extra
/// inputs of the chip, and stays unknown when nothing separates
/// a tile on a card from a block inside the processor, so an
/// unknown placement is never presented as either one.
#[must_use]
pub fn placement(vendor: GpuVendor, facts: ChipFacts) -> GpuPlacement {
    match vendor {
        GpuVendor::Nvidia => GpuPlacement::Discrete,
        GpuVendor::SystemOnChip => GpuPlacement::Integrated,
        GpuVendor::Amd => {
            if facts.northbridge_voltage {
                GpuPlacement::Integrated
            } else if facts.fan {
                GpuPlacement::Discrete
            } else {
                GpuPlacement::Unknown
            }
        }
        GpuVendor::Intel => match facts.on_root_complex {
            Some(true) => GpuPlacement::Integrated,
            Some(false) => GpuPlacement::Discrete,
            None => {
                if facts.fan {
                    GpuPlacement::Discrete
                } else {
                    GpuPlacement::Unknown
                }
            }
        }
    }
}

/// Best processor reading among the chips handed in.
///
/// The chip decides first, so a driver bound to the processor always
/// beats the board thermal zone that reports whichever node the
/// firmware happens to expose; the input decides second, so the
/// package reading beats a core.
#[must_use]
pub fn select_cpu(chips: &[ChipView<'_>]) -> Option<Pick> {
    chips
        .iter()
        .enumerate()
        .filter_map(|(chip_index, chip)| {
            let chip_rank = cpu_chip_rank(chip.chip)?;

            chip.inputs
                .iter()
                .enumerate()
                .map(|(input_index, input)| {
                    (
                        (chip_rank, cpu_input_rank(input)),
                        Pick {
                            chip:  chip_index,
                            input: input_index
                        }
                    )
                })
                .min_by_key(|(rank, _)| *rank)
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, pick)| pick)
}

/// Best graphics reading among the chips handed in.
///
/// `preferred` names the device a user pinned in the configuration,
/// matched against the vendor, the chip or the placement; an
/// entry that matches nothing is ignored rather than leaving
/// the machine without a reading.
#[must_use]
pub fn select_gpu(chips: &[ChipView<'_>], preferred: Option<&str>) -> Option<GpuPick> {
    let preferred = preferred.map(catalog::normalise);

    chips
        .iter()
        .enumerate()
        .filter_map(|(chip_index, chip)| {
            let vendor = gpu_vendor(chip.chip)?;
            let placement = placement(vendor, chip.facts);
            let pinned = preferred
                .as_deref()
                .is_some_and(|wanted| matches(wanted, chip.chip, vendor, placement));

            chip.inputs
                .iter()
                .enumerate()
                .map(|(input_index, input)| {
                    (
                        (u8::from(!pinned), placement, gpu_input_rank(input)),
                        GpuPick {
                            chip: chip_index,
                            input: input_index,
                            vendor,
                            placement
                        }
                    )
                })
                .min_by_key(|(rank, _)| *rank)
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, pick)| pick)
}

/// Reports whether a configured preference names this device.
#[must_use]
pub fn matches(
    preferred: &str,
    chip: &str,
    vendor: GpuVendor,
    placement: GpuPlacement
) -> bool {
    if preferred.is_empty() {
        return false;
    }

    let placement_name = match placement {
        GpuPlacement::Discrete => "discrete",
        GpuPlacement::Integrated => "integrated",
        GpuPlacement::Unknown => "unknown"
    };

    preferred == catalog::normalise_chip(chip)
        || preferred == vendor.as_str()
        || preferred == placement_name
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chip set of a machine, written down the way `sensors` prints it.
    struct Sensors {
        chips: Vec<(String, Vec<(String, f32)>, ChipFacts)>
    }

    impl Sensors {
        /// Builds a machine out of `("<chip> <input>", value)` pairs.
        fn from_labels(readings: &[(&str, f32)]) -> Self {
            let mut chips: Vec<(String, Vec<(String, f32)>, ChipFacts)> = Vec::new();

            for (label, value) in readings {
                let (chip, input) = match label.split_once(' ') {
                    Some((chip, input)) => (chip, input),
                    None => (*label, "")
                };

                match chips.iter_mut().find(|(name, _, _)| name == chip) {
                    Some((_, inputs, _)) => inputs.push((input.to_owned(), *value)),
                    None => chips.push((
                        chip.to_owned(),
                        vec![(input.to_owned(), *value)],
                        ChipFacts::default()
                    ))
                }
            }

            Self {
                chips
            }
        }

        /// Marks a chip as publishing the northbridge rail of an
        /// integrated part.
        fn integrated(mut self, chip: &str) -> Self {
            self.mark(chip, |facts| facts.northbridge_voltage = true);
            self
        }

        /// Marks a chip as owning a fan, the way a card does.
        fn with_fan(mut self, chip: &str) -> Self {
            self.mark(chip, |facts| facts.fan = true);
            self
        }

        /// Gives a chip an address on the bus.
        fn at(mut self, chip: &str, address: &'static str) -> Self {
            self.mark(chip, move |facts| {
                facts.on_root_complex =
                    ChipFacts::from_address(Some(address)).on_root_complex;
            });
            self
        }

        fn mark(&mut self, chip: &str, apply: impl Fn(&mut ChipFacts)) {
            if let Some((_, _, facts)) =
                self.chips.iter_mut().find(|(name, _, _)| name == chip)
            {
                apply(facts);
            }
        }

        fn views(&self) -> (Vec<Vec<&str>>, Vec<ChipFacts>) {
            let inputs = self
                .chips
                .iter()
                .map(|(_, inputs, _)| {
                    inputs.iter().map(|(label, _)| label.as_str()).collect()
                })
                .collect();
            let facts = self.chips.iter().map(|(_, _, facts)| *facts).collect();

            (inputs, facts)
        }

        fn cpu(&self) -> Option<(String, f32)> {
            let (inputs, facts) = self.views();
            let views = self.chip_views(&inputs, &facts);

            select_cpu(&views).map(|pick| self.reading(pick.chip, pick.input))
        }

        fn gpu(&self, preferred: Option<&str>) -> Option<(String, f32, GpuPlacement)> {
            let (inputs, facts) = self.views();
            let views = self.chip_views(&inputs, &facts);

            select_gpu(&views, preferred).map(|pick| {
                let (label, value) = self.reading(pick.chip, pick.input);

                (label, value, pick.placement)
            })
        }

        fn chip_views<'a>(
            &'a self,
            inputs: &'a [Vec<&'a str>],
            facts: &'a [ChipFacts]
        ) -> Vec<ChipView<'a>> {
            self.chips
                .iter()
                .enumerate()
                .map(|(index, (chip, _, _))| ChipView {
                    chip:   chip.as_str(),
                    inputs: inputs[index].as_slice(),
                    facts:  facts[index]
                })
                .collect()
        }

        fn reading(&self, chip: usize, input: usize) -> (String, f32) {
            let (name, inputs, _) = &self.chips[chip];
            let (label, value) = &inputs[input];

            (format!("{name} {label}").trim().to_owned(), *value)
        }
    }

    /// Desktop machine: AMD processor, AMD card, storage and network
    /// noise.
    fn amd_desktop() -> Sensors {
        Sensors::from_labels(&[
            ("k10temp Tctl", 56.6),
            ("k10temp Tccd1", 51.0),
            ("amdgpu edge", 47.0),
            ("amdgpu junction", 55.0),
            ("amdgpu mem", 62.0),
            ("nvme Composite", 39.9),
            ("acpitz_0 temp1", 59.0),
            ("r8169_0_c100:00 temp1", 48.5),
            ("mt7925_phy0 temp1", 49.0)
        ])
        .with_fan("amdgpu")
    }

    /// Laptop with switchable graphics: Intel processor and graphics
    /// block, NVIDIA card awake.
    fn intel_laptop_with_card() -> Sensors {
        Sensors::from_labels(&[
            ("coretemp Package id 0", 58.0),
            ("coretemp Core 0", 71.0),
            ("coretemp Core 1", 69.0),
            ("i915 temp1", 45.0),
            ("nvidia temp1", 66.0),
            ("acpitz temp1", 47.0),
            ("nvme Composite", 41.0)
        ])
        .at("i915", "0000:00:02.0")
    }

    #[test]
    fn the_processor_chip_beats_the_board_thermal_zone() {
        assert_eq!(amd_desktop().cpu(), Some(("k10temp Tctl".to_owned(), 56.6)));
    }

    #[test]
    fn the_package_reading_beats_a_single_core() {
        assert_eq!(
            intel_laptop_with_card().cpu(),
            Some(("coretemp Package id 0".to_owned(), 58.0))
        );
    }

    #[test]
    fn a_core_stands_in_when_the_chip_names_no_package() {
        let sensors =
            Sensors::from_labels(&[("acpitz temp1", 80.0), ("k10temp Tccd1", 61.0)]);

        assert_eq!(sensors.cpu(), Some(("k10temp Tccd1".to_owned(), 61.0)));
    }

    #[test]
    fn the_thermal_zone_stands_in_when_no_processor_chip_reports() {
        let sensors = Sensors::from_labels(&[
            ("amdgpu edge", 52.0),
            ("nvme Composite", 42.8),
            ("acpitz_0 temp1", 47.0)
        ]);

        assert_eq!(sensors.cpu(), Some(("acpitz_0 temp1".to_owned(), 47.0)));
    }

    #[test]
    fn storage_and_network_chips_are_never_the_processor_or_the_graphics() {
        let sensors = Sensors::from_labels(&[
            ("nvme Composite", 42.8),
            ("nvme Sensor 1", 40.8),
            ("r8169_0_c100:00 temp1", 48.5),
            ("mt7925_phy0 temp1", 49.0)
        ]);

        assert_eq!(sensors.cpu(), None);
        assert_eq!(sensors.gpu(None), None);
    }

    #[test]
    fn a_machine_without_sensors_reports_nothing() {
        let sensors = Sensors::from_labels(&[]);

        assert_eq!(sensors.cpu(), None);
        assert_eq!(sensors.gpu(None), None);
    }

    #[test]
    fn the_graphics_die_reading_wins_over_the_edge_and_the_memory() {
        let (label, value, placement) = amd_desktop().gpu(None).expect("graphics reading");

        assert_eq!((label.as_str(), value), ("amdgpu junction", 55.0));
        assert_eq!(placement, GpuPlacement::Discrete);
    }

    #[test]
    fn the_card_wins_over_the_block_inside_the_processor() {
        let (label, value, placement) = intel_laptop_with_card()
            .gpu(None)
            .expect("graphics reading");

        assert_eq!((label.as_str(), value), ("nvidia temp1", 66.0));
        assert_eq!(placement, GpuPlacement::Discrete);
    }

    #[test]
    fn the_block_inside_the_processor_is_reported_as_integrated_when_alone() {
        let sensors = Sensors::from_labels(&[
            ("k10temp Tctl", 71.6),
            ("amdgpu edge", 47.0),
            ("acpitz_0 temp1", 62.0)
        ])
        .integrated("amdgpu");

        let (label, value, placement) = sensors.gpu(None).expect("graphics reading");

        assert_eq!((label.as_str(), value), ("amdgpu edge", 47.0));
        assert_eq!(placement, GpuPlacement::Integrated);
        assert_eq!(placement.tag(), Some("iGPU"));
    }

    #[test]
    fn a_sleeping_card_leaves_the_block_inside_the_processor_and_says_so() {
        let sensors = Sensors::from_labels(&[
            ("coretemp Package id 0", 58.0),
            ("i915 temp1", 45.0),
            ("acpitz temp1", 47.0)
        ])
        .at("i915", "0000:00:02.0");

        let (label, value, placement) = sensors.gpu(None).expect("graphics reading");

        assert_eq!((label.as_str(), value), ("i915 temp1", 45.0));
        assert_eq!(placement, GpuPlacement::Integrated);
        assert_eq!(
            placement.tag(),
            Some("iGPU"),
            "the reading of the block inside the processor is never shown as the card"
        );
    }

    #[test]
    fn an_intel_card_behind_a_bridge_is_not_taken_for_the_block_in_the_processor() {
        let sensors = Sensors::from_labels(&[("xe temp1", 61.0)]).at("xe", "0000:03:00.0");

        let (_, _, placement) = sensors.gpu(None).expect("graphics reading");

        assert_eq!(placement, GpuPlacement::Discrete);
        assert_eq!(placement.tag(), None);
    }

    #[test]
    fn a_machine_with_three_devices_reports_a_card_and_can_be_pinned_to_either() {
        let sensors = Sensors::from_labels(&[
            ("i915 temp1", 45.0),
            ("amdgpu junction", 58.0),
            ("nvidia temp1", 66.0)
        ])
        .at("i915", "0000:00:02.0")
        .with_fan("amdgpu");

        let (label, _, placement) = sensors.gpu(None).expect("graphics reading");

        assert_eq!(placement, GpuPlacement::Discrete);
        assert!(label == "amdgpu junction" || label == "nvidia temp1");
        assert_eq!(
            sensors.gpu(Some("nvidia")).map(|(label, _, _)| label),
            Some("nvidia temp1".to_owned())
        );
        assert_eq!(
            sensors.gpu(Some("integrated")).map(|(label, _, _)| label),
            Some("i915 temp1".to_owned())
        );
    }

    #[test]
    fn two_cards_pick_the_one_the_configuration_names() {
        let sensors =
            Sensors::from_labels(&[("amdgpu junction", 55.0), ("nvidia temp1", 66.0)])
                .with_fan("amdgpu");

        assert_eq!(
            sensors.gpu(Some("amd")).map(|(label, _, _)| label),
            Some("amdgpu junction".to_owned())
        );
        assert_eq!(
            sensors.gpu(Some("nvidia")).map(|(label, _, _)| label),
            Some("nvidia temp1".to_owned())
        );
        assert_eq!(
            sensors.gpu(Some("integrated")).map(|(label, _, _)| label),
            Some("amdgpu junction".to_owned()),
            "a preference that matches nothing must not hide the machine"
        );
    }

    #[test]
    fn a_second_processor_package_does_not_break_the_choice() {
        let sensors = Sensors::from_labels(&[
            ("coretemp Package id 0", 58.0),
            ("coretemp_1 Package id 1", 61.0)
        ]);

        assert_eq!(
            sensors.cpu(),
            Some(("coretemp Package id 0".to_owned(), 58.0))
        );
    }

    #[test]
    fn a_chip_with_a_numeric_suffix_keeps_its_family() {
        let sensors = Sensors::from_labels(&[("acpitz_1 temp1", 44.0)]);

        assert_eq!(sensors.cpu(), Some(("acpitz_1 temp1".to_owned(), 44.0)));
    }

    #[test]
    fn an_unlabelled_input_still_reports_when_it_is_all_the_chip_has() {
        let sensors = Sensors::from_labels(&[("nvidia temp1", 66.0)]);

        assert_eq!(
            sensors.gpu(None).map(|(label, value, _)| (label, value)),
            Some(("nvidia temp1".to_owned(), 66.0))
        );
    }
}
