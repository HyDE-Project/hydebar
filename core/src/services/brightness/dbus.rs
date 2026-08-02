//! `logind` session proxy used to write the brightness level.

use std::path::Path;

use zbus::proxy;

use super::{BrightnessError, BrightnessService};

impl BrightnessService {
    pub(super) async fn set_brightness(
        conn: &zbus::Connection,
        device_path: &Path,
        value: u32
    ) -> Result<(), BrightnessError> {
        let brightness_ctrl = BrightnessCtrlProxy::new(conn)
            .await
            .map_err(BrightnessError::from)?;
        let device_name = device_path
            .file_name()
            .and_then(|d| d.to_str())
            .ok_or_else(|| {
                BrightnessError::filesystem(format!(
                    "invalid device path: {}",
                    device_path.display()
                ))
            })?;

        brightness_ctrl
            .set_brightness("backlight", device_name, value)
            .await
            .map_err(BrightnessError::from)?;

        Ok(())
    }
}

#[proxy(
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto",
    interface = "org.freedesktop.login1.Session"
)]
trait BrightnessCtrl {
    fn set_brightness(&self, subsystem: &str, name: &str, value: u32) -> zbus::Result<()>;
}
