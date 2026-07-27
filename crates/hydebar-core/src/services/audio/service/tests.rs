// TODO: Fix broken tests
#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex}
    };

    use futures::FutureExt;
    use libpulse_binding::volume::ChannelVolumes;
    use tokio::sync::mpsc;

    use super::*;
    use crate::services::audio::backend::BackendFuture;

    #[tokio::test]
    async fn commands_are_dispatched_to_backend() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut service = AudioService {
            data:      AudioData {
                server_info:       crate::services::audio::model::ServerInfo {
                    default_sink:   "sink".into(),
                    default_source: "source".into()
                },
                sinks:             vec![Device {
                    name:        "sink".into(),
                    description: String::new(),
                    volume:      ChannelVolumes::default(),
                    is_mute:     false,
                    in_use:      true,
                    ports:       vec![crate::services::audio::model::Port {
                        name:        "port".into(),
                        description: String::new(),
                        device_type: crate::services::audio::model::DeviceType::Speaker,
                        active:      true
                    }]
                }],
                sources:           vec![Device {
                    name:        "source".into(),
                    description: String::new(),
                    volume:      ChannelVolumes::default(),
                    is_mute:     false,
                    in_use:      true,
                    ports:       vec![crate::services::audio::model::Port {
                        name:        "port".into(),
                        description: String::new(),
                        device_type: crate::services::audio::model::DeviceType::Headset,
                        active:      true
                    }]
                }],
                cur_sink_volume:   0,
                cur_source_volume: 0
            },
            commander: tx
        };

        service.apply_command(AudioCommand::ToggleSinkMute);
        match rx.recv().await {
            Some(BackendCommand::SinkMute(name, true)) if name == "sink" => {}
            other => panic!("unexpected command: {other:?}")
        }

        service.apply_command(AudioCommand::ToggleSourceMute);
        match rx.recv().await {
            Some(BackendCommand::SourceMute(name, true)) if name == "source" => {}
            other => panic!("unexpected command: {other:?}")
        }
    }

    #[derive(Clone)]
    struct TestBackend {
        sequences: Arc<Mutex<VecDeque<Vec<BackendEvent>>>>,
        starts:    Arc<Mutex<usize>>
    }

    impl TestBackend {
        fn new(sequences: Vec<Vec<BackendEvent>>) -> Self {
            Self {
                sequences: Arc::new(Mutex::new(sequences.into_iter().collect())),
                starts:    Arc::new(Mutex::new(0))
            }
        }

        fn start_count(&self) -> usize {
            *self.starts.lock().unwrap()
        }
    }

    impl AudioBackend for TestBackend {
        fn spawn(&self) -> BackendFuture {
            let sequences = self.sequences.clone();
            let starts = self.starts.clone();

            Box::pin(async move {
                let events = sequences
                    .lock()
                    .unwrap()
                    .pop_front()
                    .unwrap_or_else(|| vec![BackendEvent::Error("exhausted".into())]);

                *starts.lock().unwrap() += 1;

                let (event_tx, event_rx) = mpsc::unbounded_channel();
                let (command_tx, mut command_rx) = mpsc::unbounded_channel();

                tokio::spawn(async move {
                    for event in events {
                        let _ = event_tx.send(event);
                    }
                    drop(event_tx);
                    while command_rx.recv().await.is_some() {}
                });

                Ok(BackendHandle::from_parts(event_rx, command_tx))
            })
        }
    }

    struct TestPublisher {
        sender: mpsc::UnboundedSender<ServiceEvent<AudioService>>
    }

    impl ServiceEventPublisher<AudioService> for TestPublisher {
        type SendFuture<'a>
            = futures::future::BoxFuture<'a, ()>
        where
            Self: 'a;

        fn send(&mut self, event: ServiceEvent<AudioService>) -> Self::SendFuture<'_> {
            let sender = self.sender.clone();
            async move {
                let _ = sender.send(event);
            }
            .boxed()
        }
    }

    #[tokio::test(start_paused = true)]
    #[ignore = "Timing-sensitive test - needs rework"]
    async fn service_reconnects_after_backend_error() {
        tokio::time::pause();

        let backend = TestBackend::new(vec![
            vec![BackendEvent::Error("failure".into())],
            vec![BackendEvent::Update(AudioEvent::ServerInfo(
                crate::services::audio::model::ServerInfo {
                    default_sink:   String::from("sink"),
                    default_source: String::from("source")
                }
            ))],
        ]);

        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let publisher = TestPublisher {
            sender: event_tx
        };

        let backend_clone = backend.clone();
        let listener = tokio::spawn(async move {
            let mut publisher = publisher;
            AudioService::listen_with_backend(backend_clone, &mut publisher).await;
        });

        // Expect first init event.
        let first = event_rx.recv().await.unwrap();
        assert!(matches!(first, ServiceEvent::Init(_)));

        // Advance time to allow reconnection attempts after error.
        tokio::time::advance(RECONNECT_BACKOFF).await;
        tokio::time::advance(RECONNECT_BACKOFF).await;

        // Expect an error event followed by a new init and update.
        let mut init_count = 1;
        let mut update_seen = false;
        for _ in 0..4 {
            if let Some(event) = event_rx.recv().await {
                match event {
                    ServiceEvent::Init(_) => init_count += 1,
                    ServiceEvent::Update(AudioEvent::ServerInfo(_)) => {
                        update_seen = true;
                        break;
                    }
                    _ => {}
                }
            }
        }

        assert!(
            update_seen,
            "expected server info update after reconnection"
        );
        assert_eq!(init_count, 2, "expected service to reinitialise once");
        assert_eq!(backend.start_count(), 2);

        listener.abort();
    }
}
