//! What two looks at the interface counters say about the traffic
//! between them.

use std::time::Instant;

use itertools::Itertools;
use sysinfo::Networks;

use super::NetworkData;

#[derive(Debug, Clone)]
pub(super) struct NetworkSnapshot {
    ip:                Option<String>,
    total_received:    u64,
    total_transmitted: u64,
    timestamp:         Instant
}

impl NetworkSnapshot {
    pub(super) fn capture(networks: &Networks, now: Instant) -> Option<Self> {
        let (ip, total_received, total_transmitted) = networks.iter().fold(
            (None, 0_u64, 0_u64),
            |(first_ip, received, transmitted), (_, data)| {
                let next_ip = first_ip.or_else(|| {
                    data.ip_networks()
                        .iter()
                        .sorted_by(|a, b| a.addr.cmp(&b.addr))
                        .next()
                        .map(|ip| ip.addr.to_string())
                });

                (
                    next_ip,
                    received + data.received(),
                    transmitted + data.transmitted()
                )
            }
        );

        let ip = ip?;

        Some(Self {
            ip: Some(ip),
            total_received,
            total_transmitted,
            timestamp: now
        })
    }

    pub(super) fn to_data(&self, previous: Option<&Self>) -> NetworkData {
        let elapsed = previous
            .map(|snapshot| self.timestamp.saturating_duration_since(snapshot.timestamp))
            .unwrap_or_default();
        let seconds = elapsed.as_secs();

        #[expect(
            clippy::cast_possible_truncation,
            reason = "kilobyte deltas and the seconds of a short sampling window fit u32"
        )]
        let compute_speed = |current: u64, previous_total: u64| -> u32 {
            if seconds == 0 {
                return 0;
            }

            let delta = current.saturating_sub(previous_total);
            ((delta / 1000) as u32) / (seconds as u32)
        };

        NetworkData {
            ip:             self.ip.clone().unwrap_or_else(|| "Unknown".to_string()),
            download_speed: compute_speed(
                self.total_received,
                previous.map_or(0, |snapshot| snapshot.total_received)
            ),
            upload_speed:   compute_speed(
                self.total_transmitted,
                previous.map_or(0, |snapshot| snapshot.total_transmitted)
            ),
            received:       self.total_received,
            transmitted:    self.total_transmitted,
            last_check:     self.timestamp
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn snapshot_speed_zero_when_no_elapsed() {
        let timestamp = Instant::now();
        let previous = NetworkSnapshot {
            ip: Some("127.0.0.1".to_string()),
            total_received: 1024,
            total_transmitted: 2048,
            timestamp
        };
        let snapshot = NetworkSnapshot {
            ip: Some("127.0.0.1".to_string()),
            total_received: 2048,
            total_transmitted: 4096,
            timestamp
        };

        let data = snapshot.to_data(Some(&previous));

        assert_eq!(data.download_speed, 0);
        assert_eq!(data.upload_speed, 0);
    }
}
