//! The clock: the current time in the configured formats, and nothing else.
//!
//! The calendar and the weather are modules of their own; this one only knows
//! what time it is. Its bar entry opens the calendar menu, which is
//! composition between blocks, not knowledge of each other's insides.
//!
//! One folder, three rooms: [`ticker`] runs the tick loop on the wall-clock
//! boundaries the formats call for, [`state`] folds messages in and follows
//! the format cycle, and [`module`] wires the module to the bar. The root
//! holds the state the rooms share.

use std::time::Duration;

use chrono::{DateTime, Local};
use tokio::task::JoinHandle;

use crate::{ModuleEventSender, format_cycle::FormatCycle};

mod module;
mod state;
mod ticker;

/// The moment the clock last read.
#[derive(Debug, Clone)]
pub struct ClockData {
    pub current_time: DateTime<Local>
}

impl ClockData {
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_time: Local::now()
        }
    }

    pub fn update(&mut self) {
        self.current_time = Local::now();
    }

    /// The time rendered through a `chrono` format string.
    #[must_use]
    pub fn format(&self, format: &str) -> String {
        self.current_time.format(format).to_string()
    }
}

impl Default for ClockData {
    fn default() -> Self {
        Self::new()
    }
}

/// Events emitted by the clock module.
#[derive(Debug, Clone)]
pub enum ClockEvent {
    Tick(DateTime<Local>)
}

/// What the clock reacts to.
#[derive(Debug, Clone, Copy)]
pub enum Message {
    Update,
    /// Switch to the next configured format, wrapping after the last one.
    NextFormat
}

/// The clock module.
#[derive(Debug)]
pub struct Clock {
    data:          ClockData,
    tick_interval: Duration,
    sender:        Option<ModuleEventSender<ClockEvent>>,
    task:          Option<JoinHandle<()>>,
    format:        FormatCycle,
    shown:         crate::components::crossfade::Crossfade
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            data:          ClockData::new(),
            tick_interval: Duration::from_secs(5),
            sender:        None,
            task:          None,
            format:        FormatCycle::new(),
            shown:         crate::components::crossfade::Crossfade::default()
        }
    }
}

impl Clock {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The moment the clock last read.
    #[must_use]
    pub const fn data(&self) -> &ClockData {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_data_format() {
        let data = ClockData::new();
        let formatted = data.format("%H:%M");
        assert!(formatted.contains(':'));
        assert_eq!(formatted.len(), 5);
    }
}
