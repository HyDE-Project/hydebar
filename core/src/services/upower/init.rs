//! Initial data collected when the service connects.

use std::{any::TypeId, time::Duration};

use iced::{Subscription, stream::channel};
use log::warn;
use masterror::{AppError, AppResult};
use zbus::zvariant::ObjectPath;

use super::{
    BatteryData, BatteryStatus, PowerProfile, UPowerService,
    dbus::{Battery, PowerProfilesProxy, UPowerDbus}
};
use crate::services::ServiceEvent;

impl UPowerService {
    pub fn subscription_with_id(id: TypeId) -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(id, |&_id| {
            channel(100, async |mut output| {
                Self::listen(&mut output).await;
            })
        })
    }

    pub(super) async fn initialize_data(
        conn: &zbus::Connection
    ) -> AppResult<(
        Option<(BatteryData, Vec<ObjectPath<'static>>)>,
        PowerProfile
    )> {
        let battery = Self::initialize_battery_data(conn).await?;
        let power_profile = Self::initialize_power_profile_data(conn).await;

        match (battery, power_profile) {
            (Some(battery), Ok(power_profile)) => Ok((
                Some((battery.0, battery.1.get_devices_path())),
                power_profile
            )),
            (Some(battery), Err(err)) => {
                warn!("Failed to get power profile: {err}");

                Ok((
                    Some((battery.0, battery.1.get_devices_path())),
                    PowerProfile::Unknown
                ))
            }
            (None, Ok(power_profile)) => Ok((None, power_profile)),
            (None, Err(err)) => {
                warn!("Failed to get power profile: {err}");

                Ok((None, PowerProfile::Unknown))
            }
        }
    }

    pub(super) async fn initialize_power_profile_data(
        conn: &zbus::Connection
    ) -> AppResult<PowerProfile> {
        let powerprofiles = PowerProfilesProxy::new(conn).await.map_err(|e| {
            AppError::internal(format!("Failed to create PowerProfilesProxy: {e}"))
        })?;

        let profile = powerprofiles
            .active_profile()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get active power profile: {e}")))
            .map(PowerProfile::from)?;

        Ok(profile)
    }

    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "UPower reports non-negative charge times and a 0-100 percentage"
    )]
    pub(super) async fn initialize_battery_data(
        conn: &zbus::Connection
    ) -> AppResult<Option<(BatteryData, Battery)>> {
        let upower = UPowerDbus::new(conn).await?;
        let battery = upower.get_battery_devices().await?;

        match battery {
            Some(battery) => {
                let state = battery.state().await;
                let state = match state {
                    1 => BatteryStatus::Charging(Duration::from_secs(
                        battery.time_to_full().await as u64
                    )),
                    2 => BatteryStatus::Discharging(Duration::from_secs(
                        battery.time_to_empty().await as u64
                    )),
                    4 => BatteryStatus::Full,
                    _ => BatteryStatus::Discharging(Duration::from_secs(0))
                };
                let percentage = battery.percentage().await as i64;
                let health = battery.health().await;
                let cycles = battery.cycles().await;
                let watts = battery.watts().await;
                let watt_hours = battery.watt_hours().await;

                Ok(Some((
                    BatteryData {
                        capacity: percentage,
                        status: state,
                        health,
                        cycles,
                        watts,
                        watt_hours
                    },
                    battery
                )))
            }
            _ => Ok(None)
        }
    }
}
