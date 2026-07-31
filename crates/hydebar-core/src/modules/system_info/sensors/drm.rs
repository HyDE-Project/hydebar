//! Load and memory a graphics driver publishes next to its device.
//!
//! The monitoring subsystem carries temperatures only, so utilisation
//! and video memory come from the direct rendering device the
//! same card owns. The two are paired by the device the kernel
//! links both to, which keeps a machine with several cards from
//! mixing one card's load with another's temperature.

use std::path::{Path, PathBuf};

use super::{catalog::GpuVendor, hwmon::read_number};

/// Location the kernel publishes rendering devices at.
pub const DEFAULT_ROOT: &str = "/sys/class/drm";

/// Power state of a device that is not currently in use.
const SUSPENDED: &str = "suspended";

/// One rendering device and the attributes it publishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// Driver bound to the card, such as `amdgpu` or `nvidia`.
    pub driver:         Option<String>,
    pub vendor:         Option<GpuVendor>,
    /// Device the card hangs off, shared with its monitoring chip.
    pub device:         Option<PathBuf>,
    /// Share of the card that is busy, in percent.
    pub busy:           Option<PathBuf>,
    pub memory_used:    Option<PathBuf>,
    pub memory_total:   Option<PathBuf>,
    /// Runtime power state, present once the driver allows the card to
    /// sleep.
    pub runtime_status: Option<PathBuf>
}

impl Card {
    /// Reports whether the card is asleep right now.
    ///
    /// A card that powers down between uses is the normal state of
    /// switchable graphics, and reading it would be both
    /// meaningless and enough to wake it, so the panel
    /// leaves a sleeping card alone.
    #[must_use]
    pub fn is_asleep(&self) -> bool {
        self.runtime_status
            .as_deref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .is_some_and(|status| status.trim() == SUSPENDED)
    }

    /// Share of the card that is busy, in percent.
    pub fn utilisation(&self, buffer: &mut String) -> Option<u32> {
        let busy: u32 = read_number(self.busy.as_deref()?, buffer).ok()?;

        Some(busy.min(100))
    }

    /// Video memory in use and installed, in bytes.
    pub fn memory(&self, buffer: &mut String) -> Option<(u64, u64)> {
        let used: u64 = read_number(self.memory_used.as_deref()?, buffer).ok()?;
        let total: u64 = read_number(self.memory_total.as_deref()?, buffer).ok()?;

        (total > 0).then_some((used, total))
    }
}

/// Every rendering device the kernel publishes, in a stable order.
#[must_use]
pub fn scan(root: &Path) -> Vec<Card> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };

    let mut directories: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_card_name)
        })
        .collect();
    directories.sort();

    directories.iter().map(|path| read_card(path)).collect()
}

/// Reports whether a name addresses a card rather than one of its
/// connectors.
///
/// The subsystem lists connectors beside the cards as `card0-HDMI-A-1`,
/// and a connector publishes neither load nor memory.
fn is_card_name(name: &str) -> bool {
    name.strip_prefix("card").is_some_and(|rest| {
        !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn read_card(path: &Path) -> Card {
    let device = path.join("device");
    let resolved = std::fs::canonicalize(&device).ok();

    Card {
        driver:         std::fs::canonicalize(device.join("driver")).ok().and_then(
            |driver| {
                driver
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_owned)
            }
        ),
        vendor:         std::fs::read_to_string(device.join("vendor"))
            .ok()
            .and_then(|id| super::catalog::gpu_vendor_from_pci(&id)),
        busy:           present(device.join("gpu_busy_percent")),
        memory_used:    present(device.join("mem_info_vram_used")),
        memory_total:   present(device.join("mem_info_vram_total")),
        runtime_status: present(device.join("power/runtime_status")),
        device:         resolved
    }
}

fn present(path: PathBuf) -> Option<PathBuf> {
    path.exists().then_some(path)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn card(root: &Path, name: &str) -> PathBuf {
        let device = root.join(name).join("device");
        fs::create_dir_all(&device).expect("create card directory");

        device
    }

    #[test]
    fn an_absent_subsystem_yields_no_cards() {
        let root = TempDir::new().expect("temporary root");

        assert!(scan(&root.path().join("missing")).is_empty());
    }

    #[test]
    fn connectors_are_not_mistaken_for_cards() {
        assert!(is_card_name("card0"));
        assert!(is_card_name("card1"));
        assert!(!is_card_name("card1-DP-1"));
        assert!(!is_card_name("card1-HDMI-A-1"));
        assert!(!is_card_name("renderD128"));
        assert!(!is_card_name("version"));
    }

    #[test]
    fn load_and_memory_are_read_when_the_driver_publishes_them() {
        let root = TempDir::new().expect("temporary root");
        let device = card(root.path(), "card1");
        fs::write(device.join("vendor"), "0x1002\n").expect("vendor");
        fs::write(device.join("gpu_busy_percent"), "11\n").expect("load");
        fs::write(device.join("mem_info_vram_used"), "9605545984\n").expect("used");
        fs::write(device.join("mem_info_vram_total"), "68719476736\n").expect("total");

        let cards = scan(root.path());
        let mut buffer = String::new();

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].vendor, Some(GpuVendor::Amd));
        assert_eq!(cards[0].utilisation(&mut buffer), Some(11));
        assert_eq!(
            cards[0].memory(&mut buffer),
            Some((9_605_545_984, 68_719_476_736))
        );
        assert!(!cards[0].is_asleep());
    }

    #[test]
    fn a_card_that_publishes_nothing_reports_nothing() {
        let root = TempDir::new().expect("temporary root");
        card(root.path(), "card0");

        let cards = scan(root.path());
        let mut buffer = String::new();

        assert_eq!(cards[0].utilisation(&mut buffer), None);
        assert_eq!(cards[0].memory(&mut buffer), None);
    }

    #[test]
    fn a_sleeping_card_is_recognised() {
        let root = TempDir::new().expect("temporary root");
        let device = card(root.path(), "card0");
        fs::create_dir_all(device.join("power")).expect("power directory");
        fs::write(device.join("power/runtime_status"), "suspended\n").expect("status");

        let cards = scan(root.path());

        assert!(cards[0].is_asleep());
    }
}
