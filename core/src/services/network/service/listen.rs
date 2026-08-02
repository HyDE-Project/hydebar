//! Connection lifecycle and event forwarding.

use log::error;
use tokio::time::sleep;

use super::{NetworkService, NetworkServiceError, State, gate::EventGate};
use crate::services::ServiceEventPublisher;

mod active;
mod backoff;
mod forward;
mod init;
mod throttle;

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

    pub(in crate::services::network::service) async fn start_listening<P>(
        state: State,
        publisher: &mut P,
        gate: &mut EventGate
    ) -> State
    where
        P: ServiceEventPublisher<Self> + Send
    {
        match state {
            State::Init => Self::initialize(publisher, gate).await,
            State::Active(conn, choice) => Self::run_active(conn, choice, publisher, gate).await,
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
                    sleep(backoff::reconnect_delay(failures)).await;
                }
                State::Active(..) => failures = 0,
                State::Init => {}
            }
        }
    }
}
