//! The devices the session speaks through: sound, link, radio, backlight.
//!
//! All four are worked from the control centre and read from here. The centre
//! answers with what its services last heard, so a machine without a radio or
//! without a backlight contributes no block rather than a block of zeroes.

use hydebar_core::services::{
    audio::AudioData,
    bluetooth::BluetoothState,
    network::{ActiveConnectionInfo, ConnectivityState}
};

use super::super::{Panel, push};
use crate::app::state::App;

/// The sound server: what is playing out, what is listening in.
pub fn sound(app: &App) -> Option<Panel> {
    let audio = app.control_center.audio_readings()?;
    let mut rows = vec![
        ("output".to_owned(), named(audio, true)),
        (
            "volume".to_owned(),
            level(audio.cur_sink_volume, muted(audio, true))
        ),
        ("input".to_owned(), named(audio, false)),
        (
            "input level".to_owned(),
            level(audio.cur_source_volume, muted(audio, false))
        ),
    ];

    rows.push(("outputs".to_owned(), audio.sinks.len().to_string()));
    rows.push(("inputs".to_owned(), audio.sources.len().to_string()));

    Panel::of("sound", rows)
}

/// The description of the default sink or source, by its own name.
fn named(audio: &AudioData, out: bool) -> String {
    let (devices, wanted) = if out {
        (&audio.sinks, &audio.server_info.default_sink)
    } else {
        (&audio.sources, &audio.server_info.default_source)
    };

    devices
        .iter()
        .find(|device| device.name == *wanted)
        .map_or_else(|| wanted.clone(), |device| device.description.clone())
}

/// Whether the default device of that direction is silenced.
fn muted(audio: &AudioData, out: bool) -> bool {
    let (devices, wanted) = if out {
        (&audio.sinks, &audio.server_info.default_sink)
    } else {
        (&audio.sources, &audio.server_info.default_source)
    };

    devices
        .iter()
        .find(|device| device.name == *wanted)
        .is_some_and(|device| device.is_mute)
}

/// A volume, said as a share and as a state.
fn level(volume: i32, muted: bool) -> String {
    if muted {
        return format!("{volume}%, muted");
    }

    format!("{volume}%")
}

/// The link the machine's bytes ride on.
pub fn link(app: &App) -> Option<Panel> {
    let network = app.control_center.network_readings()?;
    let mut rows = vec![(
        "state".to_owned(),
        match network.connectivity {
            ConnectivityState::Full => "connected",
            ConnectivityState::Portal => "behind a portal",
            ConnectivityState::None => "no route",
            ConnectivityState::Loss => "lost",
            ConnectivityState::Unknown => "unknown"
        }
        .to_owned()
    )];

    push(&mut rows, "network", active_name(network));
    push(&mut rows, "interface", network.link.interface.clone());
    push(
        &mut rows,
        "signal",
        network.link.signal_dbm.map(|dbm| format!("{dbm} dBm"))
    );
    push(
        &mut rows,
        "channel",
        network
            .link
            .frequency_mhz
            .map(|mhz| format!("{:.3} GHz", f64::from(mhz) / 1000.0))
    );
    push(&mut rows, "address", network.link.address.clone());
    push(&mut rows, "gateway", network.link.gateway.clone());
    push(&mut rows, "netmask", network.link.netmask.clone());

    if network.wifi_present {
        rows.push((
            "wireless".to_owned(),
            if network.wifi_enabled { "on" } else { "off" }.to_owned()
        ));
        rows.push((
            "nearby".to_owned(),
            network.wireless_access_points.len().to_string()
        ));
    }

    if network.airplane_mode {
        rows.push(("airplane mode".to_owned(), "on".to_owned()));
    }

    Panel::of("link", rows)
}

/// The name of whatever the default route rides on.
fn active_name(network: &hydebar_core::services::network::NetworkData) -> Option<String> {
    network
        .active_connections
        .iter()
        .map(|connection| match connection {
            ActiveConnectionInfo::WiFi {
                name, ..
            }
            | ActiveConnectionInfo::Wired {
                name, ..
            }
            | ActiveConnectionInfo::Vpn {
                name, ..
            } => name.clone()
        })
        .next()
}

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
