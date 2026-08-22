//! What the machine has been doing for the last few minutes.
//!
//! A reading is one moment; a reading beside the ones before it is a shape,
//! and a shape says things a number cannot. A processor at forty percent is a
//! processor at forty percent whether it has been climbing for a minute or
//! settling for one, and only the last few minutes tell those apart.
//!
//! Kept here rather than sampled here: the readings arrive when the module
//! that owns them publishes, and this only remembers them.

/// How many readings are kept.
///
/// The system module publishes about twice a minute at rest, so this is the
/// last half hour of an idle machine and the last minute of a busy one — long
/// enough for a shape to have a shape, short enough that the whole trace is a
/// handful of numbers.
const KEPT: usize = 64;

/// One reading remembered over time.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trail {
    /// The readings, oldest first.
    seen: std::collections::VecDeque<f32>
}

impl Trail {
    /// Remembers one more reading, forgetting the oldest when it is full.
    pub fn saw(&mut self, reading: f32) {
        if self.seen.len() == KEPT {
            self.seen.pop_front();
        }

        self.seen.push_back(reading);
    }

    /// The readings, oldest first.
    #[must_use]
    pub fn seen(&self) -> Vec<f32> {
        self.seen.iter().copied().collect()
    }

    /// Whether there is enough of a trail to draw a shape from.
    ///
    /// Two readings make a line, and a line is the least that says anything;
    /// one reading is the number that already stands beside it.
    #[must_use]
    pub fn is_drawable(&self) -> bool {
        self.seen.len() > 1
    }
}

/// Everything the bar remembers about the last few minutes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct History {
    /// Share of the processor that was busy.
    pub cpu:  Trail,
    /// Temperature the processor reported, in whole degrees.
    pub heat: Trail,
    /// Share of the memory that was in use.
    pub ram:  Trail
}

impl History {
    /// Folds one system sample into the trails.
    pub fn saw(&mut self, data: &hydebar_core::modules::system_info::SystemInfoData) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a whole-percent share is far below any precision limit"
        )]
        {
            self.cpu.saw(data.cpu_usage as f32);
            self.ram.saw(data.memory_usage as f32);
        }

        if let Some(heat) = data.cpu_temperature {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a temperature in whole degrees is a two figure number"
            )]
            self.heat.saw(heat as f32);
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    #[test]
    fn a_fresh_trail_has_nothing_to_draw() {
        let mut trail = Trail::default();

        assert!(!trail.is_drawable());
        trail.saw(1.0);
        assert!(!trail.is_drawable(), "one reading is not a shape");
        trail.saw(2.0);
        assert!(trail.is_drawable());
    }

    #[test]
    fn the_oldest_reading_is_forgotten_once_the_trail_is_full() {
        let mut trail = Trail::default();

        for step in 0..(KEPT + 10) {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            trail.saw(step as f32);
        }

        let seen = trail.seen();

        assert_eq!(seen.len(), KEPT);
        assert_eq!(seen[0], 10.0, "the first ten were forgotten");
        #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
        let last = (KEPT + 9) as f32;
        assert_eq!(seen[seen.len() - 1], last);
    }

    #[test]
    fn a_sample_without_a_temperature_leaves_that_trail_alone() {
        let mut history = History::default();
        let mut data = hydebar_core::modules::system_info::SystemInfoData {
            cpu_usage: 40,
            memory_usage: 60,
            ..hydebar_core::modules::system_info::SystemInfoData::default()
        };

        history.saw(&data);
        data.cpu_temperature = Some(55);
        history.saw(&data);

        assert_eq!(history.cpu.seen(), vec![40.0, 40.0]);
        assert_eq!(history.ram.seen(), vec![60.0, 60.0]);
        assert_eq!(history.heat.seen(), vec![55.0]);
    }
}
