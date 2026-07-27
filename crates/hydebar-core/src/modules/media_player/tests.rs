#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    use std::{
        num::NonZeroUsize,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering}
        },
        time::Duration
    };

    use futures::future::pending;
    use tokio::{task::yield_now, time::timeout};

    use super::*;
    use crate::{
        event_bus::{BusEvent, EventBus, ModuleEvent as BusModuleEvent},
        services::mpris::test_support::{
            ExecuteCommandCallback, StartListeningCallback, install_execute_command_override,
            install_start_listening_override
        }
    };

    async fn recv_event(receiver: &mut crate::event_bus::EventReceiver) -> BusEvent {
        loop {
            if let Some(event) = receiver
                .try_recv()
                .expect("event bus receiver should not be poisoned")
            {
                return event;
            }

            yield_now().await;
        }
    }

    struct CancellationProbe {
        flag: Arc<AtomicBool>
    }

    impl Drop for CancellationProbe {
        fn drop(&mut self) {
            self.flag.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn command_success_emits_refresh_event() {
        let listener_callback: StartListeningCallback = Arc::new(|state, _publisher| {
            let _ = state;
            Box::pin(async { pending::<Result<ListenerState, ModuleError>>().await })
        });
        let _listener_guard = install_start_listening_override(listener_callback);

        let command_callback: ExecuteCommandCallback =
            Arc::new(|_service, _command| Box::pin(async { Ok(Vec::new()) }));
        let _command_guard = install_execute_command_override(command_callback);

        let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
        let mut receiver = bus.receiver();
        let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let mut media_player = MediaPlayer::default();
        assert!(
            <MediaPlayer as Module<Message>>::register(&mut media_player, &context, ()).is_ok()
        );

        media_player.handle_command("player".to_string(), PlayerCommand::Next);

        let event = timeout(Duration::from_secs(1), recv_event(&mut receiver))
            .await
            .expect("media player event should be emitted");

        match event {
            BusEvent::Module(BusModuleEvent::MediaPlayer(Message::Event(
                ServiceEvent::Update(MprisPlayerEvent::Refresh(data))
            ))) => {
                assert!(data.is_empty());
            }
            other => panic!("unexpected event: {other:?}")
        }

        for task in media_player.tasks.drain(..) {
            task.abort();
        }
    }

    #[tokio::test]
    #[ignore = "Timing-sensitive test - needs rework"]
    async fn command_failure_emits_error_event() {
        let listener_callback: StartListeningCallback = Arc::new(|state, _publisher| {
            let _ = state;
            Box::pin(async { pending::<Result<ListenerState, ModuleError>>().await })
        });
        let _listener_guard = install_start_listening_override(listener_callback);

        let error = ModuleError::registration("command failure");
        let command_callback: ExecuteCommandCallback = Arc::new({
            let error = error.clone();
            move |_service, _command| {
                let error = error.clone();
                Box::pin(async move { Err(error) })
            }
        });
        let _command_guard = install_execute_command_override(command_callback);

        let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
        let mut receiver = bus.receiver();
        let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let mut media_player = MediaPlayer::default();
        assert!(
            <MediaPlayer as Module<Message>>::register(&mut media_player, &context, ()).is_ok()
        );

        media_player.handle_command("player".to_string(), PlayerCommand::PlayPause);

        let event = timeout(Duration::from_secs(1), recv_event(&mut receiver))
            .await
            .expect("media player event should be emitted");

        match event {
            BusEvent::Module(BusModuleEvent::MediaPlayer(Message::Event(
                ServiceEvent::Error(err)
            ))) => {
                assert_eq!(err, error);
            }
            other => panic!("unexpected event: {other:?}")
        }

        for task in media_player.tasks.drain(..) {
            task.abort();
        }
    }

    #[tokio::test]
    #[ignore = "Timing-sensitive test - needs rework"]
    async fn register_aborts_previous_listener() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let call_count = Arc::new(AtomicUsize::new(0));

        let listener_callback: StartListeningCallback = Arc::new({
            let cancelled = Arc::clone(&cancelled);
            let call_count = Arc::clone(&call_count);

            move |state, _publisher| {
                call_count.fetch_add(1, Ordering::SeqCst);
                let flag = Arc::clone(&cancelled);

                Box::pin(async move {
                    let _probe = CancellationProbe {
                        flag
                    };
                    let _ = state;
                    pending::<Result<ListenerState, ModuleError>>().await
                })
            }
        });
        let _listener_guard = install_start_listening_override(listener_callback);

        let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
        let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let mut media_player = MediaPlayer::default();
        assert!(
            <MediaPlayer as Module<Message>>::register(&mut media_player, &context, ()).is_ok()
        );
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        assert!(
            <MediaPlayer as Module<Message>>::register(&mut media_player, &context, ()).is_ok()
        );
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

        timeout(Duration::from_secs(1), async {
            loop {
                if cancelled.load(Ordering::SeqCst) {
                    break;
                }
                yield_now().await;
            }
        })
        .await
        .expect("previous listener should be cancelled");

        for task in media_player.tasks.drain(..) {
            task.abort();
        }
    }
}
