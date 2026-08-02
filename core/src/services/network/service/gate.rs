//! The events worth waking the bar for.
//!
//! Every event that reaches the bar rebuilds and repaints every surface it
//! owns, so an event carrying a value nobody can see, or a value the bar
//! already shows, costs a full repaint for nothing. A wireless daemon on a
//! populated band produces a great many of those: it reports the signal
//! strength of every radio in earshot, while the bar draws the strength of the
//! one connection it is actually on.

use std::collections::{HashMap, HashSet};

use super::super::{ActiveConnectionInfo, NetworkData, NetworkEvent};

/// Drops the events that would repaint the bar without changing it.
#[derive(Debug, Default)]
pub(super) struct EventGate {
    /// Names of the connections currently carrying traffic.
    active:    HashSet<String>,
    /// Strength last let through, per SSID.
    strengths: HashMap<String, u8>
}

impl EventGate {
    /// Opens the gate on the state the backend reported when it connected.
    pub(super) fn new(data: &NetworkData) -> Self {
        let mut gate = Self::default();
        gate.remember_active(&data.active_connections);

        gate
    }

    /// Reports whether `event` changes something the bar draws.
    ///
    /// Recording happens here as well, so a caller that admits an event has
    /// already told the gate about it.
    pub(super) fn admits(&mut self, event: &NetworkEvent) -> bool {
        match event {
            NetworkEvent::ActiveConnections(connections) => {
                self.remember_active(connections);
                self.strengths
                    .retain(|ssid, _| self.active.contains(ssid.as_str()));

                true
            }
            NetworkEvent::Strength((ssid, strength)) => {
                if !self.active.contains(ssid.as_str()) {
                    return false;
                }

                if self.strengths.get(ssid) == Some(strength) {
                    return false;
                }

                self.strengths.insert(ssid.clone(), *strength);

                true
            }
            _ => true
        }
    }

    /// Records the connections the backend reports as active.
    fn remember_active(&mut self, connections: &[ActiveConnectionInfo]) {
        self.active = connections.iter().map(ActiveConnectionInfo::name).collect();
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn wifi(name: &str, strength: u8) -> ActiveConnectionInfo {
        ActiveConnectionInfo::WiFi {
            id: name.to_owned(),
            name: name.to_owned(),
            strength
        }
    }

    fn gate() -> EventGate {
        EventGate::new(&NetworkData {
            active_connections: vec![wifi("home", 60)],
            ..NetworkData::default()
        })
    }

    #[test]
    fn a_strength_that_did_not_change_never_reaches_the_bar() {
        let mut gate = gate();

        assert!(gate.admits(&NetworkEvent::Strength(("home".to_owned(), 61))));
        assert!(
            !gate.admits(&NetworkEvent::Strength(("home".to_owned(), 61))),
            "a value already on screen must not force a repaint"
        );
        assert!(gate.admits(&NetworkEvent::Strength(("home".to_owned(), 62))));
    }

    #[test]
    fn the_strength_of_a_neighbour_never_reaches_the_bar() {
        let mut gate = gate();

        assert!(
            !gate.admits(&NetworkEvent::Strength(("someone else".to_owned(), 40))),
            "the bar draws the connection it is on, not the ones it can see"
        );
    }

    #[test]
    fn moving_to_another_network_lets_its_strength_through() {
        let mut gate = gate();
        assert!(!gate.admits(&NetworkEvent::Strength(("cafe".to_owned(), 40))));

        assert!(gate.admits(&NetworkEvent::ActiveConnections(vec![wifi("cafe", 40)])));

        assert!(gate.admits(&NetworkEvent::Strength(("cafe".to_owned(), 41))));
        assert!(!gate.admits(&NetworkEvent::Strength(("home".to_owned(), 61))));
    }

    #[test]
    fn everything_the_bar_draws_from_still_gets_through() {
        let mut gate = gate();

        assert!(gate.admits(&NetworkEvent::WiFiEnabled(false)));
        assert!(gate.admits(&NetworkEvent::AirplaneMode(true)));
        assert!(gate.admits(&NetworkEvent::WirelessAccessPoint(Vec::new())));
        assert!(gate.admits(&NetworkEvent::ScanningNearbyWifi));
    }
}
