//! Power profile commands issued by the user.

use log::error;
use masterror::AppError;

use super::{PowerProfile, UPowerEvent, UPowerService, dbus::PowerProfilesProxy};
use crate::services::{Service, ServiceEvent};

impl UPowerService {
    pub async fn run_command(self, command: PowerProfileCommand) -> ServiceEvent<Self> {
        let conn = self.conn.clone();
        let power_profile = self.power_profile;

        let powerprofiles = match PowerProfilesProxy::new(&conn)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create PowerProfilesProxy: {e}")))
        {
            Ok(proxy) => proxy,
            Err(err) => {
                error!("Failed to create PowerProfilesProxy: {err}");
                return ServiceEvent::Error(());
            }
        };

        let next_profile = match command {
            PowerProfileCommand::Toggle => match power_profile {
                PowerProfile::Balanced => {
                    if powerprofiles
                        .set_active_profile("performance")
                        .await
                        .map_err(|e| {
                            AppError::internal(format!(
                                "Failed to set power profile to performance: {e}"
                            ))
                        })
                        .is_err()
                    {
                        return ServiceEvent::Error(());
                    }
                    PowerProfile::Performance
                }
                PowerProfile::Performance => {
                    if powerprofiles
                        .set_active_profile("power-saver")
                        .await
                        .map_err(|e| {
                            AppError::internal(format!(
                                "Failed to set power profile to power-saver: {e}"
                            ))
                        })
                        .is_err()
                    {
                        return ServiceEvent::Error(());
                    }
                    PowerProfile::PowerSaver
                }
                PowerProfile::PowerSaver => {
                    if powerprofiles
                        .set_active_profile("balanced")
                        .await
                        .map_err(|e| {
                            AppError::internal(format!(
                                "Failed to set power profile to balanced: {e}"
                            ))
                        })
                        .is_err()
                    {
                        return ServiceEvent::Error(());
                    }
                    PowerProfile::Balanced
                }
                PowerProfile::Unknown => PowerProfile::Unknown
            }
        };

        ServiceEvent::Update(UPowerEvent::UpdatePowerProfile(next_profile))
    }
}

#[derive(Debug)]
pub enum PowerProfileCommand {
    Toggle
}

impl Service for UPowerService {
    type Command = PowerProfileCommand;

    fn command(&mut self, command: Self::Command) -> iced::Task<ServiceEvent<Self>> {
        let service = self.clone();

        iced::Task::perform(
            async move { Self::run_command(service, command).await },
            |event| event
        )
    }
}
