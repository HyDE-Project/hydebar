//! Journal discipline for a check that keeps failing the same way.

use log::{debug, warn};

/// Identical failures passed over between two log lines.
///
/// A mirror that is down stays down, and the check meets it once per
/// interval. Writing the same line every time buries everything else in
/// the journal, so the repetitions are counted and reported in one line
/// instead.
pub(super) const FAILURE_REPEAT: u32 = 12;

/// Writes a failure to the journal unless it is the same one all over
/// again.
pub(super) fn report(failures: &mut FailureLog, reason: &str) {
    match failures.record(reason) {
        Some(1) => warn!("updates check failed: {reason}"),
        Some(count) => warn!("updates check has failed {count} times in a row: {reason}"),
        None => debug!("updates check failed again: {reason}")
    }
}

/// Counter that keeps an unchanging failure out of the journal.
#[derive(Debug, Default)]
pub(super) struct FailureLog {
    /// The failure the run before this one reported.
    last:    Option<String>,
    /// How many times it has been reported in a row.
    repeats: u32
}

impl FailureLog {
    /// Records a failure, reporting the count when it deserves a log line.
    ///
    /// A failure that differs from the last one is always worth a line; an
    /// identical one only every [`FAILURE_REPEAT`] occurrences, so a
    /// lasting fault is visible without being the only thing in the
    /// journal.
    pub(super) fn record(&mut self, reason: &str) -> Option<u32> {
        if self.last.as_deref() == Some(reason) {
            self.repeats += 1;

            return self
                .repeats
                .is_multiple_of(FAILURE_REPEAT)
                .then_some(self.repeats);
        }

        self.last = Some(reason.to_owned());
        self.repeats = 1;

        Some(1)
    }

    /// Forgets the last failure after a check that worked.
    pub(super) fn clear(&mut self) {
        self.last = None;
        self.repeats = 0;
    }
}
