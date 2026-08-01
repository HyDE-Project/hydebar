//! What the machine can be asked about its own temperature and load.
//!
//! The panel discovers the sensors once, keeps the files it settled on, and
//! a refresh reads exactly those files through one reused buffer: no
//! walk of the subsystem and no search for a chip. The set is rebuilt
//! on a slow cadence, and at once after a read fails, so a card that
//! woke up appears and a card that went to sleep disappears without
//! anybody restarting the bar.
//!
//! A machine that publishes nothing - a virtual one, or a board whose
//! drivers expose no thermal registers - yields no readings at all.
//! That is an ordinary state: no indicator, no placeholder and nothing
//! in the log.

pub mod catalog;
pub mod drm;
pub mod hwmon;
pub mod selection;
pub mod utility;

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant}
};

use log::warn;

pub use self::catalog::{GpuPlacement, GpuVendor};
use self::{
    drm::Card,
    selection::{ChipView, select_cpu, select_gpu},
    utility::{Feed, ProcessRunner, UTILITY_INTERVAL}
};

/// How long the panel keeps a sensor set before rebuilding it.
///
/// Hardware comes and goes while the bar runs, and a walk of two small
/// sysfs trees is cheap, but not cheap enough to repeat every few
/// seconds for a set that almost never changes.
pub const DISCOVERY_INTERVAL: Duration = Duration::from_secs(30);

/// Temperature of the processor and everything known about the graphics.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Readings {
    /// Processor temperature in whole degrees Celsius.
    pub cpu:        Option<i32>,
    /// Chip and label the processor temperature is read from, so the
    /// window can say what its number measures.
    pub cpu_source: Option<String>,
    pub gpu:        Option<GpuReadings>
}

/// What the panel can say about the graphics processor it watches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuReadings {
    /// Driver behind the device, such as `amdgpu` or `nvidia`.
    pub name:         String,
    /// Chip and input the temperature is taken from, for the menu.
    pub source:       Option<String>,
    pub vendor:       GpuVendor,
    pub placement:    GpuPlacement,
    /// Temperature in whole degrees Celsius.
    pub temperature:  Option<i32>,
    /// Share of the device that is busy, in percent.
    pub utilisation:  Option<u32>,
    pub memory_used:  Option<u64>,
    pub memory_total: Option<u64>
}

impl GpuReadings {
    /// Reports whether the device answered with anything worth drawing.
    #[must_use]
    fn is_empty(&self) -> bool {
        self.temperature.is_none() && self.utilisation.is_none()
    }
}

/// Temperature input the panel settled on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Input {
    label: String,
    chip:  String,
    path:  PathBuf
}

impl Input {
    fn describe(&self) -> String {
        if self.label.is_empty() {
            self.chip.clone()
        } else {
            format!("{} {}", self.chip, self.label)
        }
    }
}

/// Graphics device the panel settled on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Gpu {
    name:      String,
    vendor:    GpuVendor,
    placement: GpuPlacement,
    input:     Option<Input>,
    card:      Option<Card>
}

/// Sensors of the machine, discovered once and read from then on.
#[derive(Debug)]
pub struct HardwareSensors {
    hwmon_root:    PathBuf,
    drm_root:      PathBuf,
    preferred_gpu: Option<String>,
    discovered_at: Option<Instant>,
    cpu:           Option<Input>,
    gpu:           Option<Gpu>,
    utility:       Option<Feed>,
    buffer:        String,
    reported:      bool
}

impl Default for HardwareSensors {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareSensors {
    /// Sensors of the machine the panel runs on.
    #[must_use]
    pub fn new() -> Self {
        Self::rooted(Path::new(hwmon::DEFAULT_ROOT), Path::new(drm::DEFAULT_ROOT))
    }

    /// Sensors published under the given subsystem roots.
    #[must_use]
    pub fn rooted(hwmon_root: &Path, drm_root: &Path) -> Self {
        Self {
            hwmon_root:    hwmon_root.to_path_buf(),
            drm_root:      drm_root.to_path_buf(),
            preferred_gpu: None,
            discovered_at: None,
            cpu:           None,
            gpu:           None,
            utility:       None,
            buffer:        String::with_capacity(32),
            reported:      false
        }
    }

    /// Pins the graphics device the panel reports on.
    ///
    /// The entry names a vendor, a driver or a placement; one that matches
    /// nothing on this machine is ignored, so a configuration written for
    /// another machine never leaves this one without a reading.
    pub fn prefer_gpu(&mut self, preferred: Option<&str>) {
        let preferred = preferred
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_owned);

        if preferred != self.preferred_gpu {
            self.preferred_gpu = preferred;
            self.discovered_at = None;
        }
    }

    /// Latest readings, rebuilding the sensor set when it is due.
    pub fn read(&mut self) -> Readings {
        if self
            .discovered_at
            .is_none_or(|discovered| discovered.elapsed() >= DISCOVERY_INTERVAL)
        {
            self.discover();
        }

        let mut state = ReadState {
            failed:   false,
            reported: &mut self.reported
        };
        let cpu = read_input(self.cpu.as_ref(), &mut self.buffer, &mut state);

        let gpu = self.gpu.as_ref().map(|gpu| {
            let temperature = read_input(gpu.input.as_ref(), &mut self.buffer, &mut state);
            let awake = gpu.card.as_ref().is_none_or(|card| !card.is_asleep());
            let utilisation = awake
                .then(|| {
                    gpu.card
                        .as_ref()
                        .and_then(|card| card.utilisation(&mut self.buffer))
                })
                .flatten();
            let memory = awake
                .then(|| {
                    gpu.card
                        .as_ref()
                        .and_then(|card| card.memory(&mut self.buffer))
                })
                .flatten();

            GpuReadings {
                name: gpu.name.clone(),
                source: gpu.input.as_ref().map(Input::describe),
                vendor: gpu.vendor,
                placement: gpu.placement,
                temperature,
                utilisation,
                memory_used: memory.map(|(used, _)| used),
                memory_total: memory.map(|(_, total)| total)
            }
        });

        let failed = state.failed;
        let gpu = self
            .complete_with_utility(gpu)
            .filter(|gpu| !gpu.is_empty());

        if failed {
            self.discovered_at = None;
        } else {
            self.reported = false;
        }

        Readings {
            cpu,
            cpu_source: self.cpu.as_ref().map(Input::describe),
            gpu
        }
    }

    /// Fills in what the kernel does not publish from the vendor utility.
    fn complete_with_utility(&self, gpu: Option<GpuReadings>) -> Option<GpuReadings> {
        let mut gpu = gpu?;
        let Some(metrics) = self
            .utility
            .as_ref()
            .filter(|feed| feed.vendor() == gpu.vendor)
            .and_then(Feed::latest)
        else {
            return Some(gpu);
        };

        gpu.temperature = gpu.temperature.or(metrics.temperature);
        gpu.utilisation = gpu.utilisation.or(metrics.utilisation);
        gpu.memory_used = gpu.memory_used.or(metrics.memory_used);
        gpu.memory_total = gpu.memory_total.or(metrics.memory_total);

        Some(gpu)
    }

    /// Rebuilds the sensor set out of what the kernel publishes right now.
    fn discover(&mut self) {
        let chips = hwmon::scan(&self.hwmon_root);
        let cards = drm::scan(&self.drm_root);
        let labels: Vec<Vec<&str>> = chips
            .iter()
            .map(|chip| {
                chip.inputs
                    .iter()
                    .map(|input| input.label.as_str())
                    .collect()
            })
            .collect();
        let views: Vec<ChipView<'_>> = chips
            .iter()
            .enumerate()
            .map(|(index, chip)| ChipView {
                chip:   chip.name.as_str(),
                inputs: labels[index].as_slice(),
                facts:  chip.facts
            })
            .collect();

        let cpu = select_cpu(&views).map(|pick| Input {
            label: chips[pick.chip].inputs[pick.input].label.clone(),
            chip:  chips[pick.chip].name.clone(),
            path:  chips[pick.chip].inputs[pick.input].path.clone()
        });

        let gpu = select_gpu(&views, self.preferred_gpu.as_deref())
            .map(|pick| {
                let chip = &chips[pick.chip];
                let input = &chip.inputs[pick.input];

                Gpu {
                    name:      chip.name.clone(),
                    vendor:    pick.vendor,
                    placement: pick.placement,
                    input:     Some(Input {
                        label: input.label.clone(),
                        chip:  chip.name.clone(),
                        path:  input.path.clone()
                    }),
                    card:      pair_card(&cards, chip.device.as_deref())
                }
            })
            .or_else(|| card_only_gpu(&cards));

        self.cpu = cpu;
        self.gpu = gpu;
        self.utility = self.utility_feed();
        self.discovered_at = Some(Instant::now());
    }

    /// Starts, keeps or stops the vendor utility behind the selected
    /// device.
    ///
    /// It is started only where the kernel publishes neither a temperature
    /// nor a load for the device, and only when the program is
    /// installed, so a machine the kernel covers never spawns a
    /// process.
    fn utility_feed(&mut self) -> Option<Feed> {
        let gpu = self.gpu.as_ref()?;
        let covered =
            gpu.input.is_some() && gpu.card.as_ref().is_some_and(|card| card.busy.is_some());

        if covered {
            return None;
        }

        if let Some(feed) = self.utility.take()
            && feed.vendor() == gpu.vendor
        {
            return Some(feed);
        }

        let utility = utility::for_vendor(gpu.vendor)?;
        utility::on_path(utility.program)?;

        let card = gpu.card.clone();

        Some(Feed::spawn(
            utility,
            ProcessRunner,
            move || card.as_ref().is_none_or(|card| !card.is_asleep()),
            UTILITY_INTERVAL
        ))
    }
}

/// State one pass over the chosen inputs leaves behind.
struct ReadState<'a> {
    /// An input that was there at discovery no longer reads.
    failed:   bool,
    /// The failure has already been written to the log.
    reported: &'a mut bool
}

/// Reading behind an input, noting a failure for the caller.
///
/// A sensor that stops answering is written to the log once and then left
/// alone: the panel reads every few seconds, and a machine whose card was
/// unplugged would otherwise fill the journal with the same line. The next
/// successful pass clears the mark, so a fault that comes back is reported
/// again.
fn read_input(
    input: Option<&Input>,
    buffer: &mut String,
    state: &mut ReadState<'_>
) -> Option<i32> {
    let input = input?;

    match hwmon::read_temperature(&input.path, buffer) {
        Ok(temperature) => Some(temperature),
        Err(error) => {
            if !*state.reported {
                warn!("sensor {} stopped reporting: {error}", input.describe());
                *state.reported = true;
            }

            state.failed = true;

            None
        }
    }
}

/// Card the kernel links to the same device as the chosen chip.
fn pair_card(cards: &[Card], device: Option<&Path>) -> Option<Card> {
    let device = device?;

    cards
        .iter()
        .find(|card| card.device.as_deref() == Some(device))
        .cloned()
}

/// Graphics device known from the rendering subsystem alone.
///
/// A driver that registers no monitoring chip still publishes a device,
/// which is what a machine reports while its vendor module lacks
/// monitoring support or while the reading comes from a vendor utility
/// instead.
fn card_only_gpu(cards: &[Card]) -> Option<Gpu> {
    let card = cards
        .iter()
        .filter(|card| card.vendor.is_some())
        .min_by_key(|card| (u8::from(card.busy.is_none()), card.driver.clone()))?;
    let vendor = card.vendor?;
    let facts = selection::ChipFacts::from_address(
        card.device
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|address| address.to_str())
    );

    Some(Gpu {
        name: card
            .driver
            .clone()
            .unwrap_or_else(|| vendor.as_str().to_owned()),
        vendor,
        placement: selection::placement(vendor, facts),
        input: None,
        card: Some(card.clone())
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    /// Sysfs of a machine, written down attribute by attribute.
    struct Machine {
        hwmon:   TempDir,
        drm:     TempDir,
        devices: TempDir
    }

    impl Machine {
        fn new() -> Self {
            Self {
                hwmon:   TempDir::new().expect("monitoring root"),
                drm:     TempDir::new().expect("rendering root"),
                devices: TempDir::new().expect("device root")
            }
        }

        /// Directory standing for one piece of hardware on the bus.
        fn device(&self, slot: &str) -> PathBuf {
            let device = self.devices.path().join(slot);
            fs::create_dir_all(&device).expect("device directory");

            device
        }

        fn chip(self, index: usize, name: &str, inputs: &[(&str, &str)]) -> Self {
            let directory = self.hwmon.path().join(format!("hwmon{index}"));
            fs::create_dir_all(&directory).expect("chip directory");
            fs::write(directory.join("name"), name).expect("chip name");

            for (position, (label, value)) in inputs.iter().enumerate() {
                let input = position + 1;
                fs::write(directory.join(format!("temp{input}_input")), value)
                    .expect("reading");

                if !label.is_empty() {
                    fs::write(directory.join(format!("temp{input}_label")), label)
                        .expect("label");
                }
            }

            self
        }

        fn integrated(self, index: usize) -> Self {
            let directory = self.hwmon.path().join(format!("hwmon{index}"));
            fs::write(directory.join("in1_label"), "vddnb").expect("rail");

            self
        }

        fn card(self, index: usize, vendor: &str, busy: Option<&str>) -> Self {
            let device = self.drm.path().join(format!("card{index}")).join("device");
            fs::create_dir_all(&device).expect("card directory");
            fs::write(device.join("vendor"), vendor).expect("vendor");

            if let Some(busy) = busy {
                fs::write(device.join("gpu_busy_percent"), busy).expect("load");
                fs::write(device.join("mem_info_vram_used"), "1073741824").expect("used");
                fs::write(device.join("mem_info_vram_total"), "8589934592").expect("total");
            }

            self
        }

        /// Attaches a chip and a card to the same piece of hardware, the
        /// way the kernel links both subsystems to one device.
        fn attach(self, slot: &str, chip: usize, card: usize) -> Self {
            let device = self.device(slot);
            let card_device = self.drm.path().join(format!("card{card}")).join("device");
            fs::write(device.join("vendor"), "0x1002").expect("vendor");

            for attribute in [
                "vendor",
                "gpu_busy_percent",
                "mem_info_vram_used",
                "mem_info_vram_total"
            ] {
                let source = card_device.join(attribute);

                if source.exists() {
                    fs::rename(&source, device.join(attribute)).expect("move attribute");
                }
            }

            fs::remove_dir_all(&card_device).expect("replace card device");
            std::os::unix::fs::symlink(&device, &card_device).expect("card link");
            std::os::unix::fs::symlink(
                &device,
                self.hwmon
                    .path()
                    .join(format!("hwmon{chip}"))
                    .join("device")
            )
            .expect("chip link");

            self
        }

        fn sensors(&self) -> HardwareSensors {
            HardwareSensors::rooted(self.hwmon.path(), self.drm.path())
        }
    }

    #[test]
    fn a_machine_without_sensors_reports_nothing() {
        let machine = Machine::new();
        let readings = machine.sensors().read();

        assert_eq!(readings, Readings::default());
    }

    #[test]
    fn the_processor_and_the_graphics_are_two_separate_readings() {
        let machine = Machine::new()
            .chip(0, "acpitz_0", &[("", "62000")])
            .chip(1, "k10temp", &[("Tctl", "71625")])
            .chip(2, "amdgpu", &[("edge", "47000")])
            .integrated(2)
            .chip(3, "nvme", &[("Composite", "39850")]);

        let readings = machine.sensors().read();
        let gpu = readings.gpu.expect("graphics readings");

        assert_eq!(readings.cpu, Some(71));
        assert_eq!(gpu.temperature, Some(47));
        assert_eq!(gpu.placement, GpuPlacement::Integrated);
        assert_eq!(gpu.source.as_deref(), Some("amdgpu edge"));
    }

    #[test]
    fn load_and_memory_join_the_temperature_of_the_same_device() {
        let machine = Machine::new()
            .chip(0, "amdgpu", &[("edge", "47000")])
            .card(1, "0x1002", Some("11"))
            .attach("0000:c5:00.0", 0, 1);

        let gpu = machine.sensors().read().gpu.expect("graphics readings");

        assert_eq!(gpu.temperature, Some(47));
        assert_eq!(gpu.utilisation, Some(11));
        assert_eq!(gpu.memory_used, Some(1024 * 1024 * 1024));
        assert_eq!(gpu.memory_total, Some(8 * 1024 * 1024 * 1024));
    }

    #[test]
    fn a_second_card_does_not_lend_its_load_to_another_device() {
        let machine = Machine::new().chip(0, "amdgpu", &[("edge", "47000")]).card(
            1,
            "0x1002",
            Some("11")
        );

        let gpu = machine.sensors().read().gpu.expect("graphics readings");

        assert_eq!(gpu.temperature, Some(47));
        assert_eq!(gpu.utilisation, None);
    }

    #[test]
    fn a_card_without_a_monitoring_chip_still_reports_its_load() {
        let machine = Machine::new()
            .chip(0, "coretemp", &[("Package id 0", "58000")])
            .card(0, "0x10de", Some("42"));

        let readings = machine.sensors().read();
        let gpu = readings.gpu.expect("graphics readings");

        assert_eq!(readings.cpu, Some(58));
        assert_eq!(gpu.temperature, None);
        assert_eq!(gpu.utilisation, Some(42));
        assert_eq!(gpu.vendor, GpuVendor::Nvidia);
        assert_eq!(gpu.placement, GpuPlacement::Discrete);
    }

    #[test]
    fn a_sleeping_card_reports_no_load() {
        let machine = Machine::new().card(0, "0x10de", Some("42"));
        let device = machine.drm.path().join("card0/device/power");
        fs::create_dir_all(&device).expect("power directory");
        fs::write(device.join("runtime_status"), "suspended").expect("status");

        assert_eq!(machine.sensors().read().gpu, None);
    }

    #[test]
    fn a_disappearing_sensor_is_rediscovered_rather_than_repeated() {
        let machine = Machine::new().chip(0, "k10temp", &[("Tctl", "56600")]);
        let mut sensors = machine.sensors();

        assert_eq!(sensors.read().cpu, Some(56));

        fs::remove_dir_all(machine.hwmon.path().join("hwmon0")).expect("remove chip");

        assert_eq!(sensors.read().cpu, None);
        assert!(
            sensors.discovered_at.is_none(),
            "a failed read forces a rescan"
        );
        assert_eq!(sensors.read().cpu, None);
    }

    #[test]
    fn a_pinned_vendor_wins_over_the_better_placement() {
        let machine = Machine::new()
            .chip(0, "amdgpu", &[("edge", "47000")])
            .integrated(0)
            .chip(1, "nvidia", &[("", "66000")]);

        let mut sensors = machine.sensors();
        assert_eq!(
            sensors.read().gpu.expect("graphics readings").vendor,
            GpuVendor::Nvidia
        );

        sensors.prefer_gpu(Some("amd"));
        let gpu = sensors.read().gpu.expect("graphics readings");

        assert_eq!(gpu.vendor, GpuVendor::Amd);
        assert_eq!(gpu.temperature, Some(47));
    }

    #[test]
    fn a_processor_thermal_zone_is_never_shown_as_graphics() {
        let machine = Machine::new().chip(0, "cpu_thermal", &[("", "51000")]);
        let readings = machine.sensors().read();

        assert_eq!(readings.cpu, Some(51));
        assert_eq!(readings.gpu, None);
    }
}
