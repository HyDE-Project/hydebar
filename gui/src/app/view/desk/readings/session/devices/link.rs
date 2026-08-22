//! What carries the session's bytes, and what it knows about the way.

use hydebar_core::services::network::{ActiveConnectionInfo, ConnectivityState};

use super::super::super::{Panel, push};
use crate::app::state::App;

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
