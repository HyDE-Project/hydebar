//! Battery and power profile values exposed by the service.

use std::time::Duration;

use crate::{components::icons::Icons, utils::IndicatorState};

#[derive(Clone, Copy, Debug)]
pub struct BatteryData {
    pub capacity: i64,
    pub status:   BatteryStatus
}

impl BatteryData {
    pub fn get_indicator_state(&self) -> IndicatorState {
        match self {
            BatteryData {
                status: BatteryStatus::Charging(_),
                ..
            } => IndicatorState::Success,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity
            } if *capacity < 20 => IndicatorState::Danger,
            _ => IndicatorState::Normal
        }
    }

    pub fn get_icon(&self) -> Icons {
        match self {
            BatteryData {
                status: BatteryStatus::Charging(_),
                ..
            } => Icons::BatteryCharging,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity
            } if *capacity < 20 => Icons::Battery0,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity
            } if *capacity < 40 => Icons::Battery1,
            BatteryData {
                status: BatteryStatus::Discharging(_),
                capacity
            } if *capacity < 60 => Icons::Battery2,
            BatteryData {
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
    fn from(power_profile: String) -> PowerProfile {
        match power_profile.as_str() {
            "balanced" => PowerProfile::Balanced,
            "performance" => PowerProfile::Performance,
            "power-saver" => PowerProfile::PowerSaver,
            _ => PowerProfile::Unknown
        }
    }
}

impl From<PowerProfile> for Icons {
    fn from(profile: PowerProfile) -> Self {
        match profile {
            PowerProfile::Balanced => Icons::Balanced,
            PowerProfile::Performance => Icons::Performance,
            PowerProfile::PowerSaver => Icons::PowerSaver,
            PowerProfile::Unknown => Icons::None
        }
    }
}
