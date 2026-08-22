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

    /// The charge across every cell that answered, averaged.
    ///
    /// [`None`] when not one of them did. A machine whose cells all failed to
    /// answer is not a machine on an empty battery, and averaging nothing over
    /// nothing said exactly that — a flat zero, which the bar drew as a
    /// critical charge and a warning nobody could act on.
    pub async fn percentage(&self) -> Option<f64> {
        let mut percentage = 0.0;
        let mut count = 0_u32;

        for device in &self.0 {
            if let Ok(share) = device.percentage().await {
                percentage += share;
                count += 1;
            }
        }

        (count > 0).then(|| percentage / f64::from(count))
    }

    /// Share of the design charge the cells can still hold, averaged.
    ///
    /// Absent on a machine whose firmware does not report it, which is not
    /// the same as a cell in perfect health.
    pub async fn health(&self) -> Option<u32> {
        let mut share = 0.0;
        let mut count = 0_u32;

        for device in &self.0 {
            if let Ok(reading) = device.capacity().await
                && reading > 0.0
            {
                share += reading;
                count += 1;
            }
        }

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a share of the design charge is bounded to 0..=100"
        )]
        (count > 0).then(|| (share / f64::from(count)).round() as u32)
    }

    /// How many times the cells have been charged through, at most.
    pub async fn cycles(&self) -> Option<i32> {
        let mut most = None;

        for device in &self.0 {
            if let Ok(cycles) = device.charge_cycles().await
                && cycles > 0
            {
                most = Some(most.map_or(cycles, |seen: i32| seen.max(cycles)));
            }
        }

        most
    }

    /// What the cells are giving or taking right now, in watts.
    pub async fn watts(&self) -> Option<f64> {
        let mut rate = 0.0;

        for device in &self.0 {
            if let Ok(reading) = device.energy_rate().await {
                rate += reading;
            }
        }

        (rate > 0.0).then_some(rate)
    }

    /// What the cells hold now and what they hold full, in watt hours.
    pub async fn watt_hours(&self) -> Option<(f64, f64)> {
        let mut now = 0.0;
        let mut full = 0.0;

        for device in &self.0 {
            if let (Ok(holding), Ok(whole)) = (device.energy().await, device.energy_full().await) {
                now += holding;
                full += whole;
            }
        }

        (full > 0.0).then_some((now, full))
    }

    /// How long the cells have left, added over every one that answered.
    pub async fn time_to_empty(&self) -> i64 {
        let mut time = 0_i64;

        for device in &self.0 {
            if let Ok(left) = device.time_to_empty().await {
                time = time.saturating_add(left);
            }
        }

        time
    }

    /// How long the cells need, added over every one that answered.
    pub async fn time_to_full(&self) -> i64 {
        let mut time = 0_i64;

        for device in &self.0 {
            if let Ok(left) = device.time_to_full().await {
                time = time.saturating_add(left);
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
