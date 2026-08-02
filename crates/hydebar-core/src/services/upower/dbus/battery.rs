//! Aggregated readout over every battery device `UPower` reports.

use zbus::zvariant::ObjectPath;

use super::proxies::DeviceProxy;

pub struct Battery(pub(super) Vec<DeviceProxy<'static>>);

impl Battery {
    pub async fn state(&self) -> i32 {
        let mut charging = false;
        let mut discharging = false;

        for device in &self.0 {
            if let Ok(state) = device.state().await {
                match state {
                    1 => {
                        charging = true;
                    }
                    2 => {
                        discharging = true;
                    }
                    _ => {}
                }
            }
        }

        if charging {
            1
        } else if discharging {
            2
        } else {
            4
        }
    }

    pub async fn percentage(&self) -> f64 {
        let mut percentage = 0.0;
        let mut count = 0;

        for device in &self.0 {
            if let Ok(p) = device.percentage().await {
                percentage += p;
                count += 1;
            }
        }

        percentage / f64::from(count)
    }

    pub async fn time_to_empty(&self) -> i64 {
        let mut time = 0;

        for device in &self.0 {
            if let Ok(t) = device.time_to_empty().await {
                time += t;
            }
        }

        time
    }

    pub async fn time_to_full(&self) -> i64 {
        let mut time = 0;

        for device in &self.0 {
            if let Ok(t) = device.time_to_full().await {
                time += t;
            }
        }

        time
    }

    pub fn get_devices_path(self) -> Vec<ObjectPath<'static>> {
        self.0
            .into_iter()
            .map(|device| device.inner().path().to_owned())
            .collect()
    }
}
