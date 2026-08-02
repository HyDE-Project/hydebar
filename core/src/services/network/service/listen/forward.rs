//! Event forwarding from a backend stream to the publisher.

use iced::futures::{Stream, StreamExt};
use masterror::AppResult;

use super::{
    super::{NetworkEvent, NetworkService, gate::EventGate},
    throttle::LinkThrottle
};
use crate::services::{ServiceEvent, ServiceEventPublisher};

impl NetworkService {
    /// Whether `event` moves the link enough to re-read its kernel facts.
    ///
    /// Connection changes always do — a roam changes the frequency and the
    /// address may follow. Strength and connectivity ticks only refresh the
    /// wireless numbers, arrive every few seconds on a live link, and each
    /// read costs process spawns — so those go through `throttle` instead of
    /// spawning on every tick. Scans and password prompts leave the link
    /// exactly where it was.
    pub(super) fn moves_the_link(event: &NetworkEvent, throttle: &mut LinkThrottle) -> bool {
        match event {
            NetworkEvent::ActiveConnections(_)
            | NetworkEvent::WirelessDevice {
                ..
            } => true,
            NetworkEvent::Strength(_) | NetworkEvent::Connectivity(_) => {
                throttle.admits(std::time::Instant::now())
            }
            _ => false
        }
    }

    /// Publishes a fresh read of the link's kernel facts.
    pub(in crate::services::network::service) async fn publish_link_details<P>(publisher: &mut P)
    where
        P: ServiceEventPublisher<Self> + Send
    {
        let details = crate::services::network::link::read().await;
        let () = publisher
            .send(ServiceEvent::Update(NetworkEvent::LinkDetails(details)))
            .await;
    }

    pub(in crate::services::network::service) async fn consume_network_events<S, P>(
        mut events: S,
        publisher: &mut P,
        gate: &mut EventGate
    ) -> AppResult<()>
    where
        S: Stream<Item = AppResult<NetworkEvent>> + Unpin,
        P: ServiceEventPublisher<Self> + Send
    {
        let mut throttle = LinkThrottle::default();

        while let Some(event) = events.next().await {
            let event = event?;
            let exit_loop = matches!(event, NetworkEvent::WirelessDevice { .. });

            if gate.admits(&event) {
                let refresh_link = Self::moves_the_link(&event, &mut throttle);
                let () = publisher.send(ServiceEvent::Update(event)).await;

                if refresh_link {
                    Self::publish_link_details(publisher).await;
                }
            }

            if exit_loop {
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{super::throttle::LinkThrottle, NetworkEvent, NetworkService};

    #[test]
    fn connection_changes_re_read_the_link() {
        let mut throttle = LinkThrottle::default();

        assert!(NetworkService::moves_the_link(
            &NetworkEvent::ActiveConnections(Vec::new()),
            &mut throttle
        ));
        assert!(NetworkService::moves_the_link(
            &NetworkEvent::WirelessDevice {
                wifi_present:           true,
                wireless_access_points: Vec::new()
            },
            &mut throttle
        ));
    }

    #[test]
    fn scans_and_prompts_leave_the_link_alone() {
        let mut throttle = LinkThrottle::default();

        assert!(!NetworkService::moves_the_link(
            &NetworkEvent::ScanningNearbyWifi,
            &mut throttle
        ));
        assert!(!NetworkService::moves_the_link(
            &NetworkEvent::RequestPasswordForSSID("home".to_owned()),
            &mut throttle
        ));
        assert!(!NetworkService::moves_the_link(
            &NetworkEvent::WiFiEnabled(true),
            &mut throttle
        ));
    }

    #[test]
    fn strength_ticks_are_throttled_to_one_read_per_window() {
        let mut throttle = LinkThrottle::default();
        let tick = NetworkEvent::Strength(("home".to_owned(), 70));

        assert!(NetworkService::moves_the_link(&tick, &mut throttle));
        assert!(
            !NetworkService::moves_the_link(&tick, &mut throttle),
            "a second tick inside the window spawns nothing"
        );
    }

    #[test]
    fn connection_changes_ignore_the_throttle() {
        let mut throttle = LinkThrottle::default();
        let _ = throttle.admits(std::time::Instant::now());

        assert!(
            NetworkService::moves_the_link(
                &NetworkEvent::ActiveConnections(Vec::new()),
                &mut throttle
            ),
            "a roam or reconnect always re-reads, window or not"
        );
    }
}
