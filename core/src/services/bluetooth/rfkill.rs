//! `rfkill` soft-block probing and change monitoring.

use iced::futures::{Stream, StreamExt};
use inotify::{Inotify, WatchMask};
use masterror::{AppError, AppResult};
use tokio::process::Command;

use super::BluetoothService;

impl BluetoothService {
    /// Reports whether the bluetooth radio is soft blocked according to
    /// `rfkill`.
    ///
    /// # Errors
    ///
    /// Returns an error when the `rfkill` command cannot be executed or its
    /// output is not valid UTF-8.
    pub async fn check_rfkill_soft_block() -> AppResult<bool> {
        let output = Command::new("rfkill")
            .arg("list")
            .arg("bluetooth")
            .output()
            .await?;

        let output = String::from_utf8(output.stdout).map_err(|e| {
            AppError::deserialization(format!("Failed to parse rfkill output: {e}"))
        })?;

        Ok(output.contains("Soft blocked: yes"))
    }

    /// Watches `/dev/rfkill` and yields a unit item on every modification.
    ///
    /// # Errors
    ///
    /// Returns an error when the inotify instance cannot be created or the
    /// watch on `/dev/rfkill` cannot be added.
    pub fn listen_rfkill_soft_block_changes() -> AppResult<impl Stream<Item = ()>> {
        let inotify = Inotify::init()?;

        inotify.watches().add("/dev/rfkill", WatchMask::MODIFY)?;

        let buffer = [0; 512];
        Ok(inotify.into_event_stream(buffer)?.map(|_| {}))
    }
}
