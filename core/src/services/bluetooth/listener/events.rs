//! Every source that can change what bluetooth reports, merged into one.

use iced::futures::{Stream, StreamExt, stream::select_all, stream_select};
use masterror::{AppError, AppResult};

use super::super::{
    BluetoothService,
    dbus::{BatteryProxy, BluetoothDbus}
};

impl BluetoothService {
    #[expect(
        clippy::needless_continue,
        reason = "the continue lives inside the stream_select macro expansion"
    )]
    pub(super) async fn events(
        conn: &zbus::Connection
    ) -> AppResult<impl Stream<Item = ()> + use<>> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        let interface_changed = stream_select!(
            bluetooth
                .bluez
                .receive_interfaces_added()
                .await
                .map_err(|e| AppError::internal(format!(
                    "Failed to receive interfaces added: {e}"
                ),),)?
                .map(|_| {}),
            bluetooth
                .bluez
                .receive_interfaces_removed()
                .await
                .map_err(|e| AppError::internal(format!(
                    "Failed to receive interfaces removed: {e}"
                ),),)?
                .map(|_| {}),
        )
        .boxed();

        let combined = match bluetooth.adapter.as_ref() {
            Some(adapter) => {
                let powered = adapter.receive_powered_changed().await.map(|_| {});
                let rfkill = Self::listen_rfkill_soft_block_changes()?;
                let devices = bluetooth.devices().await?;

                let mut batteries = Vec::new();
                for device in devices.iter().filter(|d| d.battery.is_some()) {
                    let battery = BatteryProxy::builder(bluetooth.bluez.inner().connection())
                        .path(device.path.clone())
                        .map_err(|e| {
                            AppError::internal(format!("Failed to set battery path: {e}"))
                        })?
                        .build()
                        .await
                        .map_err(|e| {
                            AppError::internal(format!("Failed to build battery proxy: {e}"))
                        })?;
                    batteries.push(battery.receive_percentage_changed().await.map(|_| {}));
                }

                stream_select!(interface_changed, powered, rfkill, select_all(batteries)).boxed()
            }
            _ => interface_changed
        };

        Ok(combined)
    }
}
