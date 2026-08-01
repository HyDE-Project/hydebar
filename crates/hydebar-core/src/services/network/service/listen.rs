//! Connection lifecycle and event forwarding.

use std::time::Duration;

use iced::futures::{Stream, StreamExt, TryFutureExt};
use log::{debug, error, info};
use masterror::AppResult;
use tokio::time::sleep;

use super::{
    super::{
        NetworkData, NetworkEvent, NetworkService, NetworkServiceError,
        backend::{NetworkBackend, iwd::IwdDbus, network_manager::NetworkDbus}
    },
    BackendChoice, State,
    gate::EventGate
};
use crate::services::{ServiceEvent, ServiceEventPublisher};

impl NetworkService {
    /// Records a recoverable error on the network service state.
    ///
    /// # Examples
    /// ```
    /// use std::ops::Deref;
    ///
    /// use hydebar_core::services::network::{NetworkService, NetworkServiceError};
    ///
    /// fn inspect(service: &NetworkService) -> Option<&NetworkServiceError> {
    ///     service.deref().last_error.as_ref()
    /// }
    ///
    /// # fn exercise(mut service: NetworkService) {
    /// service.apply_error(NetworkServiceError::new("unreachable"));
    /// assert!(inspect(&service).is_some());
    /// # }
    /// ```
    pub fn apply_error(&mut self, error: NetworkServiceError) {
        self.data.last_error = Some(error);
    }

    /// Whether `event` moves the link enough to re-read its kernel facts.
    ///
    /// Connection changes always do — a roam changes the frequency and the
    /// address may follow. Strength and connectivity ticks only refresh the
    /// wireless numbers, arrive every few seconds on a live link, and each
    /// read costs process spawns — so those go through `throttle` instead of
    /// spawning on every tick. Scans and password prompts leave the link
    /// exactly where it was.
    fn moves_the_link(event: &NetworkEvent, throttle: &mut LinkThrottle) -> bool {
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
    pub(super) async fn publish_link_details<P>(publisher: &mut P)
    where
        P: ServiceEventPublisher<Self> + Send
    {
        let details = super::super::link::read().await;
        let () = publisher
            .send(ServiceEvent::Update(NetworkEvent::LinkDetails(details)))
            .await;
    }

    pub(super) async fn consume_network_events<S, P>(
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
            let exit_loop = matches!(
                event,
                NetworkEvent::WirelessDevice {
                    ..
                }
            );

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

    #[expect(
        clippy::too_many_lines,
        reason = "one state machine with an arm per listener phase; splitting the arms would separate them from the retry flow"
    )]
    pub(super) async fn start_listening<P>(
        state: State,
        publisher: &mut P,
        gate: &mut EventGate
    ) -> State
    where
        P: ServiceEventPublisher<Self> + Send
    {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn) => {
                    info!("Connecting to backend");
                    let maybe_backend: Result<(NetworkData, BackendChoice), _> =
                        match NetworkDbus::new(&conn)
                            .and_then(|nm| async move { nm.initialize_data().await })
                            .await
                        {
                            Ok(data) => {
                                info!("NetworkManager service initialized");
                                Ok((data, BackendChoice::NetworkManager))
                            }
                            Err(err) => {
                                info!(
                                    "Failed to initialize NetworkManager. Falling back to iwd. Error: {err}"
                                );
                                match IwdDbus::new(&conn)
                                    .and_then(|iwd| async move { iwd.initialize_data().await })
                                    .await
                                {
                                    Ok(data) => {
                                        info!("IWD service initialized");
                                        Ok((data, BackendChoice::Iwd))
                                    }
                                    Err(err) => {
                                        error!("Failed to initialize network service: {err}");
                                        Err(err)
                                    }
                                }
                            }
                        };
                    info!("Connected");

                    match maybe_backend {
                        Ok((data, choice)) => {
                            info!("Network service initialized");
                            *gate = EventGate::new(&data);
                            let () = publisher
                                .send(ServiceEvent::Init(Self {
                                    data,
                                    conn: conn.clone(),
                                    backend_choice: choice
                                }))
                                .await;
                            Self::publish_link_details(publisher).await;
                            State::Active(conn, choice)
                        }
                        Err(err) => {
                            if err.is::<zbus::Error>() {
                                error!("Failed to connect to system bus: {err}");
                            } else {
                                error!("Failed to initialize network service: {err}");
                            }
                            let error = NetworkServiceError::from(err);
                            let () = publisher.send(ServiceEvent::Error(error)).await;
                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus: {err}");
                    let error = NetworkServiceError::new(format!(
                        "Failed to connect to system bus: {err}"
                    ));
                    let () = publisher.send(ServiceEvent::Error(error)).await;

                    State::Error
                }
            },
            State::Active(conn, choice) => {
                info!("Listening for network events");

                match choice {
                    BackendChoice::NetworkManager => {
                        let nm = match NetworkDbus::new(&conn).await {
                            Ok(nm) => nm,
                            Err(e) => {
                                error!("Failed to create NetworkDbus: {e}");
                                let error = NetworkServiceError::from(e);
                                let () = publisher.send(ServiceEvent::Error(error)).await;
                                return State::Error;
                            }
                        };

                        match nm.subscribe_events().await {
                            Ok(events) => {
                                match Self::consume_network_events(events, publisher, gate).await {
                                    Ok(()) => {
                                        debug!("Network service exit events stream");
                                        State::Active(conn, choice)
                                    }
                                    Err(err) => {
                                        error!("Network event stream error: {err}");
                                        let error = NetworkServiceError::from(err);
                                        let () = publisher.send(ServiceEvent::Error(error)).await;
                                        State::Error
                                    }
                                }
                            }
                            Err(err) => {
                                error!("Failed to listen for network events: {err}");
                                let error = NetworkServiceError::from(err);
                                let () = publisher.send(ServiceEvent::Error(error)).await;

                                State::Error
                            }
                        }
                    }
                    BackendChoice::Iwd => {
                        let iwd = match IwdDbus::new(&conn).await {
                            Ok(iwd) => iwd,
                            Err(err) => {
                                error!("Failed to create IwdDbus: {err}");
                                let error = NetworkServiceError::from(err);
                                let () = publisher.send(ServiceEvent::Error(error)).await;
                                return State::Error;
                            }
                        };
                        match iwd.subscribe_events().await {
                            Ok(mut event_s) => {
                                let mut throttle = LinkThrottle::default();

                                while let Some(events) = event_s.next().await {
                                    for event in events {
                                        if gate.admits(&event) {
                                            let refresh_link =
                                                Self::moves_the_link(&event, &mut throttle);
                                            let () =
                                                publisher.send(ServiceEvent::Update(event)).await;

                                            if refresh_link {
                                                Self::publish_link_details(publisher).await;
                                            }
                                        }
                                    }
                                }

                                debug!("Network service exit events stream");

                                State::Active(conn, choice)
                            }
                            Err(err) => {
                                error!("Failed to listen for network events: {err}");
                                let error = NetworkServiceError::from(err);
                                let () = publisher.send(ServiceEvent::Error(error)).await;

                                State::Error
                            }
                        }
                    }
                }
            }
            State::Error => {
                error!("Network service error");

                State::Init
            }
        }
    }

    pub async fn listen<P>(publisher: &mut P)
    where
        P: ServiceEventPublisher<Self> + Send
    {
        let mut state = State::Init;
        let mut failures: u32 = 0;
        let mut gate = EventGate::default();

        loop {
            state = Self::start_listening(state, publisher, &mut gate).await;

            match state {
                State::Error => {
                    failures = failures.saturating_add(1);
                    sleep(reconnect_delay(failures)).await;
                }
                State::Active(..) => failures = 0,
                State::Init => {}
            }
        }
    }
}

/// Keeps the kernel link reads to one per window however often signals tick.
#[derive(Debug, Default)]
struct LinkThrottle {
    last: Option<std::time::Instant>
}

impl LinkThrottle {
    /// Shortest gap between two throttled link reads.
    const WINDOW: Duration = Duration::from_secs(10);

    /// Whether a read may go out at `now`, claiming the window when it may.
    fn admits(&mut self, now: std::time::Instant) -> bool {
        if self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) < Self::WINDOW)
        {
            return false;
        }

        self.last = Some(now);

        true
    }
}

/// Shortest pause between two connection attempts.
const RECONNECT_MIN_DELAY: Duration = Duration::from_secs(1);

/// Longest pause between two connection attempts.
const RECONNECT_MAX_DELAY: Duration = Duration::from_secs(64);

/// Pause before reconnecting after `failures` consecutive failed attempts.
///
/// A machine running neither `NetworkManager` nor iwd fails the D-Bus connection
/// immediately, so a fixed one second retry becomes a permanent one hertz
/// wakeup that logs an error every time. Doubling the delay up to a minute
/// keeps a transient failure recovering within a second while an absent backend
/// settles into a cost the idle bar does not notice.
fn reconnect_delay(failures: u32) -> Duration {
    let shift = failures.saturating_sub(1).min(u32::BITS - 1);

    RECONNECT_MIN_DELAY
        .saturating_mul(1u32 << shift)
        .min(RECONNECT_MAX_DELAY)
}

#[cfg(test)]
mod link_refresh_tests {
    use super::*;

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
    fn the_throttle_window_reopens_after_it_passes() {
        let mut throttle = LinkThrottle::default();
        let start = std::time::Instant::now();

        assert!(throttle.admits(start));
        assert!(!throttle.admits(start + Duration::from_secs(5)));
        assert!(throttle.admits(start + LinkThrottle::WINDOW));
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

#[cfg(test)]
mod backoff_tests {
    use super::{RECONNECT_MAX_DELAY, RECONNECT_MIN_DELAY, reconnect_delay};

    #[test]
    fn the_first_failure_retries_after_the_shortest_delay() {
        assert_eq!(reconnect_delay(1), RECONNECT_MIN_DELAY);
    }

    #[test]
    fn consecutive_failures_double_the_delay() {
        assert_eq!(reconnect_delay(2), RECONNECT_MIN_DELAY * 2);
        assert_eq!(reconnect_delay(3), RECONNECT_MIN_DELAY * 4);
        assert_eq!(reconnect_delay(4), RECONNECT_MIN_DELAY * 8);
    }

    #[test]
    fn a_backend_that_never_appears_stops_at_the_longest_delay() {
        assert_eq!(reconnect_delay(100), RECONNECT_MAX_DELAY);
        assert_eq!(reconnect_delay(u32::MAX), RECONNECT_MAX_DELAY);
    }
}
