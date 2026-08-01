//! Battery and power profile values exposed by the service.

use std::time::Duration;

use crate::{components::icons::Icons, utils::IndicatorState};

#[derive(Clone, Copy, Debug)]
pub struct BatteryData {
    pub capacity: i64,
    pub status:   BatteryStatus
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
                capacity
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
                capacity
            } if *capacity < 20 => Icons::Battery0,
            Self {
                status: BatteryStatus::Discharging(_),
                capacity
            } if *capacity < 40 => Icons::Battery1,
            Self {
                status: BatteryStatus::Discharging(_),
                capacity
            } if *capacity < 60 => Icons::Battery2,
            Self {
                status: BatteryStatus::Discharging(_),
                capacity
            } if *capacity < 80 => Icons::Battery3,
            _ => Icons::Battery4
        }
    }
}

#[derive(Debug, Clone)]
pub enum UPowerEvent {
    UpdateBattery(BatteryData),
    NoBattery,
    UpdatePowerProfile(PowerProfile)
}

#[derive(Copy, Clone, Debug)]
pub enum BatteryStatus {
    Charging(Duration),
    Discharging(Duration),
    Full
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerProfile {
    Balanced,
    Performance,
    PowerSaver,
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
