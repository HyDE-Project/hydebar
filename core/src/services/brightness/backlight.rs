//! Sysfs and udev access to the backlight device.

use std::{
    fs,
    path::{Path, PathBuf}
};

use log::debug;
use tokio::io::{Interest, unix::AsyncFd};

use super::{BrightnessData, BrightnessError, BrightnessService};

impl BrightnessService {
    fn get_max_brightness(device_path: &Path) -> Result<u32, BrightnessError> {
        let path = device_path.join("max_brightness");
        let contents = fs::read_to_string(&path)
            .map_err(|err| BrightnessError::filesystem(format!("{}: {err}", path.display())))?;
        let value = contents
            .trim()
            .parse::<u32>()
            .map_err(|err| BrightnessError::parse(format!("{}: {err}", path.display())))?;

        Ok(value)
    }

    pub(super) fn get_actual_brightness(device_path: &Path) -> Result<u32, BrightnessError> {
        let path = device_path.join("actual_brightness");
        let contents = fs::read_to_string(&path)
            .map_err(|err| BrightnessError::filesystem(format!("{}: {err}", path.display())))?;
        let value = contents
            .trim()
            .parse::<u32>()
            .map_err(|err| BrightnessError::parse(format!("{}: {err}", path.display())))?;

        Ok(value)
    }

    pub(super) fn initialize_data(device_path: &Path) -> Result<BrightnessData, BrightnessError> {
        let max_brightness = Self::get_max_brightness(device_path)?;
        let actual_brightness = Self::get_actual_brightness(device_path)?;

        debug!("Max brightness: {max_brightness}, current brightness: {actual_brightness}");

        Ok(BrightnessData {
            current: actual_brightness,
            max:     max_brightness
        })
    }

    pub(super) fn resolve_device_path(
        device_path: Option<PathBuf>
    ) -> Result<PathBuf, BrightnessError> {
        device_path.ok_or(BrightnessError::MissingDevice)
    }

    /// Builds an async file descriptor watching udev backlight events.
    ///
    /// # Errors
    ///
    /// Returns an error when the udev monitor cannot be created, the
    /// backlight subsystem filter cannot be applied, or the socket cannot be
    /// registered with the async runtime.
    pub fn backlight_monitor_listener() -> Result<AsyncFd<udev::MonitorSocket>, BrightnessError> {
        let builder = udev::MonitorBuilder::new().map_err(BrightnessError::from)?;
        let builder = builder
            .match_subsystem("backlight")
            .map_err(BrightnessError::from)?;
        let socket = builder.listen().map_err(BrightnessError::from)?;

        AsyncFd::with_interest(socket, Interest::READABLE | Interest::WRITABLE)
            .map_err(BrightnessError::from)
    }

    pub(super) fn backlight_enumerate() -> Result<Vec<udev::Device>, BrightnessError> {
        let mut enumerator = udev::Enumerator::new().map_err(BrightnessError::from)?;
        enumerator
            .match_subsystem("backlight")
            .map_err(BrightnessError::from)?;

        Ok(enumerator
            .scan_devices()
            .map_err(BrightnessError::from)?
            .collect())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::super::{BrightnessError, BrightnessService};

    #[test]
    fn resolve_device_path_without_device_fails() {
        let result = BrightnessService::resolve_device_path(None);
        assert!(matches!(result, Err(BrightnessError::MissingDevice)));
    }
}
