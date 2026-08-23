//! Battery and power profile values exposed by the service.

use std::time::Duration;

use crate::{components::icons::Icons, utils::IndicatorState};

#[derive(Clone, Copy, Debug)]
pub struct BatteryData {
    /// Charge left, as a share of full.
    pub capacity:   i64,
    /// What the battery is doing, and for how long.
    pub status:     BatteryStatus,
    /// Share of the design charge the cell can still hold, in percent.
    pub health:     Option<u32>,
    /// How many times the cell has been charged through.
    pub cycles:     Option<i32>,
    /// What the cell is giving or taking right now, in watts.
    pub watts:      Option<f64>,
    /// What the cell holds now and what it holds full, in watt hours.
    pub watt_hours: Option<(f64, f64)>
}

impl BatteryData {
    #[must_use]
    pub const fn get_indicator_state(&self) -> IndicatorState {
        match self {
            Self {
                status: BatteryStatus::Charging(_),
                ..
            } => IndicatorState::Success,
            Self {
                status: BatteryStatus::Discharging(_),
                capacity,
                ..
            } if *capacity < 20 => IndicatorState::Danger,
            _ => IndicatorState::Normal
        }
    }

    #[must_use]
    pub const fn get_icon(&self) -> Icons {
        match self {
            Self {
                status: BatteryStatus::Charging(_),
                ..
            } => Icons::BatteryCharging,
            Self {
                status: BatteryStatus::Discharging(_),
                capacity,
                ..
            } if *capacity < 20 => Icons::Battery0,
            Self {
                status: BatteryStatus::Discharging(_),
                capacity,
                ..
            } if *capacity < 40 => Icons::Battery1,
            Self {
                status: BatteryStatus::Discharging(_),
                capacity,
                ..
            } if *capacity < 60 => Icons::Battery2,
            Self {
                status: BatteryStatus::Discharging(_),
                capacity,
                ..
            } if *capacity < 80 => Icons::Battery3,
            _ => Icons::Battery4
        }
    }
}

#[derive(Debug, Clone)]
pub enum UPowerEvent {
    /// A fresh battery reading.
    UpdateBattery(BatteryData),
    /// The machine has no battery to read.
    NoBattery,
    /// The power profile changed.
    UpdatePowerProfile(PowerProfile)
}

#[derive(Copy, Clone, Debug)]
pub enum BatteryStatus {
    /// Taking charge, full in this long.
    Charging(Duration),
    /// Giving charge, empty in this long.
    Discharging(Duration),
    /// Charged, and holding.
    Full
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerProfile {
    /// Neither speed nor endurance favoured.
    Balanced,
    /// Speed favoured over endurance.
    Performance,
    /// Endurance favoured over speed.
    PowerSaver,
    /// The daemon has not said.
    #[default]
    Unknown
}

impl From<String> for PowerProfile {
    fn from(power_profile: String) -> Self {
        match power_profile.as_str() {
            "balanced" => Self::Balanced,
            "performance" => Self::Performance,
            "power-saver" => Self::PowerSaver,
            _ => Self::Unknown
        }
    }
}

impl From<PowerProfile> for Icons {
    fn from(profile: PowerProfile) -> Self {
        match profile {
            PowerProfile::Balanced => Self::Balanced,
            PowerProfile::Performance => Self::Performance,
            PowerProfile::PowerSaver => Self::PowerSaver,
            PowerProfile::Unknown => Self::None
        }
    }
}
