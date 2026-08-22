//! Per-access-point signal subscriptions: device state and signal strength.

use iced::futures::StreamExt;
use log::debug;
use masterror::{AppError, AppResult};

use super::{
    super::{
        NetworkDbus,
        proxies::{AccessPointProxy, DeviceProxy}
    },
    EventStream
};
use crate::services::{
    bus::bus_failure,
    network::{AccessPoint, DeviceState, NetworkEvent}
};

impl<'a> NetworkDbus<'a> {
    /// One stream per access point surfacing password requests on its device.
    ///
    /// # Errors
    ///
    /// Returns an error when a device proxy cannot be configured or built.
    pub(super) async fn device_state_events(
        &'a self,
        wireless_access_points: &[AccessPoint]
    ) -> AppResult<Vec<EventStream<'a>>> {
        let conn = self.0.inner().connection();

        let mut device_state_changes = Vec::with_capacity(wireless_access_points.len());
        for access_point in wireless_access_points {
            let device_proxy = DeviceProxy::builder(conn)
                .path(access_point.device_path.clone())
                .map_err(|e| bus_failure("Failed to set DeviceProxy path", &e))?
                .build()
                .await
                .map_err(|e| bus_failure("Failed to build DeviceProxy", &e))?;

            let ssid = access_point.ssid.clone();
            device_state_changes.push(
                device_proxy
                    .receive_state_changed()
                    .await
                    .then({
                        let ssid = ssid.clone();
                        move |state| {
                            let ssid = ssid.clone();
                            async move {
                                let value =
                                    state.get().await.map(DeviceState::from).map_err(|e| {
                                        AppError::internal(format!(
                                            "Failed to get device state: {e}"
                                        ))
                                    })?;
                                if value == DeviceState::NeedAuth {
                                    debug!("Request password for ssid {ssid}");
                                    Ok(Some(NetworkEvent::RequestPasswordForSSID(ssid)))
                                } else {
                                    Ok(None)
                                }
                            }
                        }
                    })
                    .filter_map(|result| async move { result.transpose() })
                    .boxed()
            );
        }

        Ok(device_state_changes)
    }

    /// One stream per access point carrying its signal strength updates.
    ///
    /// # Errors
    ///
    /// Returns an error when an access point proxy cannot be configured or
    /// built.
    pub(super) async fn strength_events(
        &'a self,
        wireless_access_points: Vec<AccessPoint>
    ) -> AppResult<Vec<EventStream<'a>>> {
        let conn = self.0.inner().connection();

        let mut strength_changes_streams = Vec::with_capacity(wireless_access_points.len());
        for access_point in wireless_access_points {
            let ssid = access_point.ssid.clone();
            let proxy = AccessPointProxy::builder(conn)
                .path(access_point.path.clone())
                .map_err(|e| bus_failure("Failed to set AccessPointProxy path", &e))?
                .build()
                .await
                .map_err(|e| bus_failure("Failed to build AccessPointProxy", &e))?;

            strength_changes_streams.push(
                proxy
                    .receive_strength_changed()
                    .await
                    .then({
                        let ssid = ssid.clone();
                        move |signal| {
                            let ssid = ssid.clone();
                            async move {
                                let value = signal.get().await.map_err(|e| {
                                    AppError::internal(format!(
                                        "Failed to get signal strength: {e}"
                                    ))
                                })?;
                                debug!("Strength changed value: {ssid}, {value}");
                                Ok(NetworkEvent::Strength((ssid, value)))
                            }
                        }
                    })
                    .boxed()
            );
        }

        Ok(strength_changes_streams)
    }
}
