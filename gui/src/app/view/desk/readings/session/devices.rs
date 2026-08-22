//! The devices the session speaks through: sound, link, radio, backlight.
//!
//! All four are worked from the control centre and read from here. The centre
//! answers with what its services last heard, so a machine without a radio or
//! without a backlight contributes no block rather than a block of zeroes.
//!
//! Two rooms: [`sound`] is what the session plays and listens through, and
//! [`link`] is what carries its bytes. The radio and the backlight are one
//! reading each and stand here.

mod link;
mod sound;

use hydebar_core::services::bluetooth::BluetoothState;
pub use link::link;
pub use sound::sound;

use super::super::Panel;
use crate::app::state::App;

/// The radio and whatever is paired to it.
pub fn radio(app: &App) -> Option<Panel> {
    let bluetooth = app.control_center.bluetooth_readings()?;

    if matches!(bluetooth.state, BluetoothState::Unavailable) {
        return None;
    }

    let connected = bluetooth
        .devices
        .iter()
        .filter(|device| device.connected)
        .count();

    let mut rows = vec![
        (
            "adapter".to_owned(),
            match bluetooth.state {
                BluetoothState::Active => "on",
                BluetoothState::Inactive => "off",
                BluetoothState::Unavailable => "absent"
            }
            .to_owned()
        ),
        ("paired".to_owned(), bluetooth.devices.len().to_string()),
        ("connected".to_owned(), connected.to_string()),
    ];

    for device in bluetooth.devices.iter().filter(|device| device.connected) {
        rows.push((
            device.name.clone(),
            device
                .battery
                .map_or_else(|| "connected".to_owned(), |charge| format!("{charge}%"))
        ));
    }

    Panel::of("bluetooth", rows)
}

/// The backlight, on a screen the session can dim.
pub fn screen(app: &App) -> Option<Panel> {
    let brightness = app.control_center.brightness_readings()?;

    if brightness.max == 0 {
        return None;
    }

    let share = brightness.current * 100 / brightness.max;

    Panel::of(
        "backlight",
        vec![
            ("brightness".to_owned(), format!("{share}%")),
            (
                "steps".to_owned(),
                format!("{} of {}", brightness.current, brightness.max)
            ),
        ]
    )
}
