//! Unit tests for the network service state machine.

use std::time::Duration;

use iced::futures::{StreamExt, channel::mpsc, stream};
use masterror::AppError;
use tokio::time::timeout;

use super::{
    super::{NetworkEvent, NetworkService, NetworkServiceError},
    State
};
use crate::services::{ServiceEvent, ServiceEventPublisher};

#[tokio::test]
async fn consume_network_events_stops_on_error() {
    let (mut sender, mut receiver) = mpsc::channel(4);

    let events = stream::iter(vec![
        Ok(NetworkEvent::WiFiEnabled(true)),
        Err(AppError::internal("boom")),
        Ok(NetworkEvent::WiFiEnabled(false)),
    ]);

    let result = NetworkService::consume_network_events(events, &mut sender).await;
    assert!(result.is_err(), "expected error from stream consumption");

    let first_event = receiver.next().await;
    assert!(
        matches!(
            first_event,
            Some(ServiceEvent::Update(NetworkEvent::WiFiEnabled(true)))
        ),
        "unexpected event: {first_event:?}"
    );

    drop(sender);
    assert!(
        receiver.next().await.is_none(),
        "no further events expected"
    );
}

#[tokio::test]
async fn state_error_transitions_to_init_after_delay() {
    let (mut sender, _receiver) = mpsc::channel(1);

    let state = timeout(
        Duration::from_secs(2),
        NetworkService::start_listening(State::Error, &mut sender)
    )
    .await
    .expect("network listener should complete after delay");
    assert!(matches!(state, State::Init));
}
