//! The battery readout: capacity, charge and power profile in the bar.
//!
//! One folder, three rooms: [`data`] maps the `UPower` readings onto the state
//! the bar draws, [`state`] folds service events in and [`view`] paints the
//! bar entry. The root holds the state the rooms share.

use crate::services::{ServiceEvent, upower::UPowerService};

mod data;
mod state;
mod view;

pub use data::{BatteryData, BatteryEvent, BatteryIcon, IndicatorState, PowerProfile};

/// Message type for GUI communication
#[derive(Debug, Clone)]
pub enum Message {
    Event(Box<ServiceEvent<UPowerService>>)
}

/// Battery monitoring module
#[derive(Debug, Default)]
pub struct Battery {
    data:  Option<BatteryData>,
    shown: crate::components::crossfade::Crossfade
}

impl Battery {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns current battery data if available
    #[must_use]
    pub const fn data(&self) -> Option<&BatteryData> {
        self.data.as_ref()
    }
}
