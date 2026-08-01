//! Assembly of the privacy event stream from its sources.

use std::path::Path;

use iced::futures::{FutureExt, StreamExt, select, stream::pending};
use log::{error, info, warn};

use super::{
    PrivacyData, PrivacyEvent, PrivacyEventPublisher, PrivacyService, State, WEBCAM_DEVICE_PATH,
    error::PrivacyError,
    inotify::{WebcamEventSource, WebcamWatcher},
    pipewire::{PipewireEventSource, PipewireListener}
};
use crate::services::ServiceEvent;

impl PrivacyService {
    pub(super) async fn emit_event<P>(
        publisher: &mut P,
        event: ServiceEvent<Self>
    ) -> Result<(), PrivacyError>
    where
        P: PrivacyEventPublisher
    {
        publisher.send(event).await
    }

    pub(crate) async fn start_listening<P>(
        state: State,
        publisher: &mut P
    ) -> Result<State, PrivacyError>
    where
        P: PrivacyEventPublisher + Send
    {
        let pipewire = PipewireListener;
        let webcam = WebcamWatcher::new(Path::new(WEBCAM_DEVICE_PATH));
        Self::start_listening_with_sources(state, publisher, &pipewire, &webcam).await
    }

    #[expect(
        clippy::future_not_send,
        reason = "the test doubles driving this are not Send; the production caller awaits it on one task"
    )]
    pub(super) async fn start_listening_with_sources<P, Pipewire, Webcam>(
        state: State,
        publisher: &mut P,
        pipewire_source: &Pipewire,
        webcam_source: &Webcam
    ) -> Result<State, PrivacyError>
    where
        P: PrivacyEventPublisher,
        Pipewire: PipewireEventSource,
        Webcam: WebcamEventSource
    {
        match state {
            State::Init => {
                let pipewire = pipewire_source.subscribe().await?;
                let webcam = match webcam_source.subscribe().await {
                    Ok(stream) => stream,
                    Err(err @ PrivacyError::WebcamUnavailable) => {
                        warn!("{err}");
                        pending::<PrivacyEvent>().boxed()
                    }
                    Err(err) => return Err(err)
                };

                let data = PrivacyData::new();
                Self::emit_event(
                    publisher,
                    ServiceEvent::Init(Self {
                        data
                    })
                )
                .await?;

                Ok(State::Active {
                    pipewire,
                    webcam
                })
            }
            State::Active {
                mut pipewire,
                mut webcam
            } => {
                info!("Listening for privacy events");

                let mut webcam_pin = webcam.as_mut();
                let mut webcam_future = webcam_pin.next().fuse();

                select! {
                    value = pipewire.recv().fuse() => {
                        if let Some(event) = value {
                            Self::emit_event(publisher, ServiceEvent::Update(event)).await?;
                        } else {
                            error!("PipeWire listener exited unexpectedly");
                            return Err(PrivacyError::channel(
                                "pipewire listener closed unexpectedly",
                            ));
                        }
                    }
                    value = webcam_future => {
                        if let Some(event) = value {
                            Self::emit_event(publisher, ServiceEvent::Update(event)).await?;
                        } else {
                            error!("Webcam listener exited unexpectedly");
                            return Err(PrivacyError::channel(
                                "webcam listener closed unexpectedly",
                            ));
                        }
                    }
                };

                Ok(State::Active {
                    pipewire,
                    webcam
                })
            }
        }
    }
}
