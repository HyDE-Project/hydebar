//! From `UPower` readings to the state the battery entry draws.

use std::time::Duration;

use crate::components::icons::Icons;

/// Battery icon type based on capacity and charging state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryIcon {
    /// Taking charge, at this share of full.
    Charging(u8),
    /// Giving charge, at this share of full.
    Discharging(u8),
    /// Charged, and holding.
    Full,
    /// The daemon has not said.
    Unknown
}

impl From<BatteryIcon> for Icons {
    fn from(icon: BatteryIcon) -> Self {
        match icon {
            BatteryIcon::Charging(_) => Self::BatteryCharging,
            BatteryIcon::Discharging(capacity) => match capacity {
                0..=20 => Self::Battery0,
                21..=40 => Self::Battery1,
                41..=60 => Self::Battery2,
                61..=80 => Self::Battery3,
                _ => Self::Battery4
            },
            BatteryIcon::Full => Self::Battery4,
            BatteryIcon::Unknown => Self::Battery0
        }
    }
}

/// Power management profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerProfile {
    /// Neither speed nor endurance favoured.
    #[default]
    Balanced,
    /// Speed favoured over endurance.
    Performance,
    /// Endurance favoured over speed.
    PowerSaver,
    /// The daemon has not said.
    Unknown
}

impl From<crate::services::upower::PowerProfile> for PowerProfile {
    fn from(profile: crate::services::upower::PowerProfile) -> Self {
        match profile {
            crate::services::upower::PowerProfile::PowerSaver => Self::PowerSaver,
            crate::services::upower::PowerProfile::Balanced => Self::Balanced,
            crate::services::upower::PowerProfile::Performance => Self::Performance,
            crate::services::upower::PowerProfile::Unknown => Self::Unknown
        }
    }
}

impl From<PowerProfile> for Icons {
    fn from(profile: PowerProfile) -> Self {
        match profile {
            PowerProfile::Performance => Self::Performance,
            PowerProfile::Balanced | PowerProfile::Unknown => Self::Balanced,
            PowerProfile::PowerSaver => Self::PowerSaver
        }
    }
}

/// Visual indicator state for battery status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorState {
    /// Nothing to say about it.
    Normal,
    /// Worth a glance.
    Warning,
    /// Worth acting on now.
    Danger,
    /// In good order.
    Success
}

/// Complete battery state information for rendering
#[derive(Debug, Clone)]
pub struct BatteryData {
    /// Charge left, as a share of full.
    pub capacity:        u8,
    /// Whether the battery is taking charge.
    pub charging:        bool,
    /// The glyph this reading is drawn with.
    pub icon:            BatteryIcon,
    /// How long the charge is expected to last.
    pub time_remaining:  Option<Duration>,
    /// The power profile in force.
    pub power_profile:   PowerProfile,
    /// How much attention the reading deserves.
    pub indicator_state: IndicatorState,
    /// What the cell can still hold against what it was sold with.
    ///
    /// A worn cell reports a full charge of a smaller cell, so a machine that
    /// runs half as long as it did is telling the truth twice over. Absent on
    /// firmware that does not report it.
    pub health:          Option<u32>,
    /// How many times the cell has been charged through.
    pub cycles:          Option<i32>,
    /// What the cell is giving or taking right now, in watts.
    pub watts:           Option<f64>,
    /// What the cell holds now and what it holds full, in watt hours.
    pub watt_hours:      Option<(f64, f64)>
}

impl BatteryData {
    #[must_use]
    pub const fn new(
        capacity: u8,
        charging: bool,
        time_remaining: Option<Duration>,
        power_profile: PowerProfile
    ) -> Self {
        let icon = if charging {
            if capacity >= 100 {
                BatteryIcon::Full
            } else {
                BatteryIcon::Charging(capacity)
            }
        } else {
            BatteryIcon::Discharging(capacity)
        };

        let indicator_state = if charging || capacity >= 100 {
            IndicatorState::Success
        } else if capacity <= 10 {
            IndicatorState::Danger
        } else if capacity <= 20 {
            IndicatorState::Warning
        } else {
            IndicatorState::Normal
        };

        Self {
            capacity,
            charging,
            icon,
            time_remaining,
            power_profile,
            indicator_state,
            health: None,
            cycles: None,
            watts: None,
            watt_hours: None
        }
    }

    /// The same reading with what the cell knows about its own wear.
    #[must_use]
    pub const fn worn(
        mut self,
        health: Option<u32>,
        cycles: Option<i32>,
        watts: Option<f64>,
        watt_hours: Option<(f64, f64)>
    ) -> Self {
        self.health = health;
        self.cycles = cycles;
        self.watts = watts;
        self.watt_hours = watt_hours;
        self
    }
}

/// Events emitted by battery module
#[derive(Debug, Clone)]
pub enum BatteryEvent {
    /// The reading moved.
    StatusChanged(BatteryData),
    /// The power profile changed.
    ProfileChanged(PowerProfile),
    /// The charge fell past the point worth a warning.
    LowBattery(u8),
    /// The charge fell past the point worth acting on.
    CriticalBattery(u8)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn battery_data_critical_state() {
        let data = BatteryData::new(5, false, None, PowerProfile::default());
        assert_eq!(data.indicator_state, IndicatorState::Danger);
    }

    #[test]
    fn battery_data_warning_state() {
        let data = BatteryData::new(15, false, None, PowerProfile::default());
        assert_eq!(data.indicator_state, IndicatorState::Warning);
    }

    #[test]
    fn battery_data_charging_success() {
        let data = BatteryData::new(50, true, None, PowerProfile::default());
        assert_eq!(data.indicator_state, IndicatorState::Success);
    }

    #[test]
    fn battery_icon_charging() {
        let data = BatteryData::new(50, true, None, PowerProfile::default());
        assert!(matches!(data.icon, BatteryIcon::Charging(50)));
    }

    #[test]
    fn battery_icon_discharging() {
        let data = BatteryData::new(75, false, None, PowerProfile::default());
        assert!(matches!(data.icon, BatteryIcon::Discharging(75)));
    }
}
