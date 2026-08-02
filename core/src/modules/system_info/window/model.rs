//! Every readout of the window, stated as data before it is drawn.
//!
//! The statement is pure: the height measurement and the drawing both
//! walk this list, so the two cannot disagree about what is shown.
//!
//! The vocabulary lives here; the sections themselves are stated next
//! door, in [`processor`] and [`sections`].

use crate::components::icons::Icons;

mod processor;
mod sections;

pub use processor::{cpu_temperature_section, processor_section, processor_section_full};
pub use sections::{
    footnotes, graphics_section, memory_section, network_section, sections, storage_section
};

/// Fill of a usage meter, judged against the shares people read as
/// trouble.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterLevel {
    Calm,
    Busy,
    Critical
}

/// Level a meter filled to `percent` draws in.
///
/// The buckets are fixed rather than configurable: the window is a
/// diagnostic surface, and four fifths full is where a pool starts
/// being worth a look whatever thresholds the bar indicators carry.
#[must_use]
pub const fn meter_level(percent: u32) -> MeterLevel {
    if percent >= 95 {
        MeterLevel::Critical
    } else if percent >= 80 {
        MeterLevel::Busy
    } else {
        MeterLevel::Calm
    }
}

/// Share of `total` behind `used`, in percent.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "the ratio times 100 is bounded to 0..=100 and fits u32; f64 blurs only fractions below the whole percent shown"
)]
pub fn share(used: u64, total: u64) -> u32 {
    if total == 0 {
        return 0;
    }

    ((used as f64 / total as f64) * 100.0) as u32
}

/// One line of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// A named value.
    Fact { label: String, value: String },
    /// A named value with a usage meter drawn under it.
    Meter {
        label:   String,
        value:   String,
        percent: u32
    }
}

/// A titled group of rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub icon:  Icons,
    pub title: &'static str,
    /// Line naming the exact source of the readings, when one
    /// matters.
    pub note:  Option<String>,
    pub rows:  Vec<Row>
}

fn fact(label: &str, value: String) -> Row {
    Row::Fact {
        label: label.to_owned(),
        value
    }
}

fn meter(label: &str, value: String, percent: u32) -> Row {
    Row::Meter {
        label: label.to_owned(),
        value,
        percent
    }
}

/// A pool spelled out with the share the meter next to it draws.
fn pool(used: u64, total: u64, percent: u32) -> String {
    format!("{} ({percent}%)", super::super::view::used_of_total(used, total))
}

/// A frequency stated in MHz, spelled in GHz.
fn gigahertz(mhz: u32) -> String {
    format!("{:.2}", f64::from(mhz) / 1000.0)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_share_is_truncated_not_rounded() {
        assert_eq!(share(1, 3), 33);
        assert_eq!(share(2, 3), 66);
    }

    #[test]
    fn a_pool_fuller_than_itself_reads_over_a_hundred() {
        assert_eq!(share(3, 2), 150);
    }
}
