//! Reading the kernel monitoring subsystem straight out of sysfs.
//!
//! Enumeration walks the subsystem once and keeps the path of every
//! input it may later read, so a refresh is a read of the files
//! that were already chosen rather than another walk. The root
//! is a parameter, so the walk is exercised against a
//! written-down directory tree instead of the machine the tests
//! happen to run on.

use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf}
};

use super::selection::ChipFacts;

/// Location the kernel publishes the monitoring subsystem at.
pub const DEFAULT_ROOT: &str = "/sys/class/hwmon";

/// Voltage rail that only a graphics block inside a processor exposes.
const NORTHBRIDGE_RAIL: &str = "vddnb";

/// One temperature input of a chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemperatureInput {
    /// Label the driver gave the input, empty when it labelled none.
    pub label: String,
    /// File the reading is taken from.
    pub path:  PathBuf
}

/// One monitoring chip with everything the selection needs to judge it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chip {
    /// Name the driver registered, such as `k10temp` or `amdgpu`.
    pub name:   String,
    pub inputs: Vec<TemperatureInput>,
    pub facts:  ChipFacts,
    /// Device the chip hangs off, resolved through its `device` link.
    ///
    /// Two subsystems describing the same piece of hardware agree on
    /// this path, which is how a temperature is paired with
    /// the utilisation the graphics driver publishes
    /// elsewhere.
    pub device: Option<PathBuf>
}

/// Every chip the monitoring subsystem publishes, in a stable order.
///
/// A machine without the subsystem, such as a virtual one, yields an
/// empty list: that is an ordinary state and not a failure, so
/// nothing is logged.
#[must_use]
pub fn scan(root: &Path) -> Vec<Chip> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut directories: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    directories.sort();

    directories
        .iter()
        .filter_map(|path| read_chip(path))
        .collect()
}

fn read_chip(directory: &Path) -> Option<Chip> {
    let name = read_trimmed(&directory.join("name"))?;
    let device = fs::canonicalize(directory.join("device")).ok();
    let mut inputs = Vec::new();
    let mut facts = ChipFacts::from_address(
        device
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|address| address.to_str())
    );

    let mut files: Vec<PathBuf> = fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    files.sort();

    for file in &files {
        let Some(file_name) = file.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if let Some(index) = temperature_index(file_name) {
            inputs.push(TemperatureInput {
                label: read_trimmed(&directory.join(format!("temp{index}_label")))
                    .unwrap_or_default(),
                path:  file.clone()
            });
        } else if file_name.starts_with("fan") && file_name.ends_with("_input") {
            facts.fan = true;
        } else if file_name.starts_with("in")
            && file_name.ends_with("_label")
            && read_trimmed(file).is_some_and(|label| label.eq_ignore_ascii_case(NORTHBRIDGE_RAIL))
        {
            facts.northbridge_voltage = true;
        }
    }

    if inputs.is_empty() {
        return None;
    }

    Some(Chip {
        name,
        inputs,
        facts,
        device
    })
}

/// Index of the temperature input a file name addresses.
fn temperature_index(file_name: &str) -> Option<&str> {
    let index = file_name.strip_prefix("temp")?.strip_suffix("_input")?;

    (!index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())).then_some(index)
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|content| content.trim().to_owned())
}

/// Temperature behind an input, in whole degrees Celsius.
///
/// `buffer` is handed in so a refresh allocates nothing. The subsystem
/// publishes millidegrees, yet a handful of drivers publish degrees, so
/// a reading small enough to be nonsense as millidegrees is
/// taken as degrees. The value is truncated rather than
/// rounded, the same way every other reading the module shows
/// is, so two indicators never disagree by a degree.
///
/// # Errors
///
/// Returns an error when the attribute cannot be read or does not parse
/// as a number.
pub fn read_temperature(path: &Path, buffer: &mut String) -> io::Result<i32> {
    let raw: i64 = read_number(path, buffer)?;

    Ok(to_degrees(raw))
}

/// Whole number behind a sysfs attribute.
///
/// # Errors
///
/// Returns an error when the attribute cannot be read or does not parse
/// as a number.
pub fn read_number<T>(path: &Path, buffer: &mut String) -> io::Result<T>
where
    T: std::str::FromStr
{
    buffer.clear();
    File::open(path)?.read_to_string(buffer)?;

    buffer
        .trim()
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "attribute is not a number"))
}

/// Folds a raw reading onto whole degrees Celsius.
#[expect(
    clippy::cast_possible_truncation,
    reason = "hwmon readings folded to whole degrees Celsius sit far inside i32"
)]
const fn to_degrees(raw: i64) -> i32 {
    if raw.abs() >= 1000 {
        (raw / 1000) as i32
    } else {
        raw as i32
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn write(path: &Path, contents: &str) {
        fs::write(path, contents).expect("write attribute");
    }

    fn chip_directory(root: &Path, index: usize, name: &str) -> PathBuf {
        let directory = root.join(format!("hwmon{index}"));
        fs::create_dir_all(&directory).expect("create chip directory");
        write(&directory.join("name"), name);

        directory
    }

    #[test]
    fn an_absent_subsystem_yields_no_chips() {
        let root = TempDir::new().expect("temporary root");

        assert!(scan(&root.path().join("missing")).is_empty());
    }

    #[test]
    fn labels_inputs_and_extra_rails_are_read() {
        let root = TempDir::new().expect("temporary root");

        let processor = chip_directory(root.path(), 0, "k10temp");
        write(&processor.join("temp1_input"), "71625\n");
        write(&processor.join("temp1_label"), "Tctl\n");

        let graphics = chip_directory(root.path(), 1, "amdgpu");
        write(&graphics.join("temp1_input"), "47000\n");
        write(&graphics.join("temp1_label"), "edge\n");
        write(&graphics.join("in1_label"), "vddnb\n");

        let zone = chip_directory(root.path(), 2, "acpitz_0");
        write(&zone.join("temp1_input"), "59000\n");

        let chips = scan(root.path());

        assert_eq!(chips.len(), 3);
        assert_eq!(chips[0].name, "k10temp");
        assert_eq!(chips[0].inputs[0].label, "Tctl");
        assert!(!chips[0].facts.northbridge_voltage);
        assert!(chips[1].facts.northbridge_voltage);
        assert_eq!(chips[2].inputs[0].label, "");
    }

    #[test]
    fn a_chip_without_a_temperature_is_left_out() {
        let root = TempDir::new().expect("temporary root");

        let power = chip_directory(root.path(), 0, "BAT0");
        write(&power.join("in0_input"), "12000");

        assert!(scan(root.path()).is_empty());
    }

    #[test]
    fn a_fan_marks_a_chip_as_owning_its_own_cooling() {
        let root = TempDir::new().expect("temporary root");

        let graphics = chip_directory(root.path(), 0, "amdgpu");
        write(&graphics.join("temp1_input"), "60000");
        write(&graphics.join("temp1_label"), "edge");
        write(&graphics.join("fan1_input"), "1200");

        let chips = scan(root.path());

        assert!(chips[0].facts.fan);
    }

    #[test]
    fn readings_arrive_in_whole_degrees_in_both_published_forms() {
        assert_eq!(to_degrees(71625), 71);
        assert_eq!(to_degrees(47000), 47);
        assert_eq!(to_degrees(56), 56);
        assert_eq!(to_degrees(-2000), -2);
        assert_eq!(to_degrees(0), 0);
    }

    #[test]
    fn a_reading_is_taken_from_the_file_it_was_discovered_at() {
        let root = TempDir::new().expect("temporary root");

        let processor = chip_directory(root.path(), 0, "coretemp");
        write(&processor.join("temp1_input"), "58000\n");
        write(&processor.join("temp1_label"), "Package id 0\n");

        let chips = scan(root.path());
        let mut buffer = String::new();

        assert_eq!(
            read_temperature(&chips[0].inputs[0].path, &mut buffer).expect("reading"),
            58
        );
    }

    #[test]
    fn a_missing_file_is_an_error_the_caller_can_act_on() {
        let root = TempDir::new().expect("temporary root");
        let mut buffer = String::new();

        assert!(read_temperature(&root.path().join("gone"), &mut buffer).is_err());
    }
}
