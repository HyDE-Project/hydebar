//! Unit tests for the tray module.

use super::super::tray::{CommandFactory, ListenerSpawner, TrayModule};

impl TrayModule {
    fn with_factories(listener_spawner: ListenerSpawner, command_factory: CommandFactory) -> Self {
        Self {
            service: None,
            submenus: Vec::new(),
            sender: None,
            runtime: None,
            listener_handles: Vec::new(),
            listener_spawner,
            command_factory
        }
    }
}

use std::{
    future::pending,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
    time::Duration
};

use tokio::{runtime::Handle, task::yield_now, time::timeout};

use super::{
    super::tray::TrayMessage,
    module::{default_command_factory, default_listener_spawner}
};
use crate::{
    ModuleContext,
    event_bus::{BusEvent, EventBus, ModuleEvent},
    modules::Module,
    services::{
        ServiceEvent,
        tray::{TrayCommand, TrayEvent}
    }
};

#[test]
#[ignore = "Timing-sensitive test - needs rework"]
fn aborts_existing_listener_on_reregistration() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("runtime");
    let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
    let context = ModuleContext::new(bus.sender(), runtime.handle().clone());

    let (tx, mut rx) = tokio::sync::oneshot::channel();
    let cancellation = Arc::new(Mutex::new(Some(tx)));
    let cancellation_spawner = Arc::clone(&cancellation);

    let listener_spawner: ListenerSpawner = Arc::new(move |_, handle: Handle| {
        let cancellation = Arc::clone(&cancellation_spawner);

        handle.spawn(async move {
            struct CancellationProbe {
                signal: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>
            }

            impl Drop for CancellationProbe {
                fn drop(&mut self) {
                    if let Some(sender) = self.signal.lock().expect("cancellation lock").take() {
                        let _ = sender.send(());
                    }
                }
            }

            let _probe = CancellationProbe {
                signal: cancellation
            };
            pending::<()>().await;
        })
    });

    let mut module = TrayModule::with_factories(listener_spawner, default_command_factory());

    <TrayModule as Module<()>>::register(&mut module, &context, ()).expect("first registration");
    <TrayModule as Module<()>>::register(&mut module, &context, ()).expect("second registration");

    runtime
        .block_on(async {
            timeout(Duration::from_secs(2), async {
                loop {
                    if rx.try_recv().is_ok() {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
        })
        .expect("listener aborted");
}

#[test]
fn publishes_command_results_via_event_bus() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("runtime");
    let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
    let sender = bus.sender();
    let mut receiver = bus.receiver();
    let context = ModuleContext::new(sender, runtime.handle().clone());

    let listener_spawner: ListenerSpawner = Arc::new(|_, handle: Handle| handle.spawn(async {}));
    let command_factory: CommandFactory = Arc::new(|_, command| match command {
        TrayCommand::MenuSelected(name, _) => {
            let layout = crate::services::tray::dbus::Layout(
                1,
                crate::services::tray::dbus::LayoutProps {
                    children_display: None,
                    label:            Some("Updated".into()),
                    type_:            None,
                    toggle_type:      None,
                    toggle_state:     None
                },
                Vec::new()
            );

            Some(Box::pin(async move {
                ServiceEvent::Update(TrayEvent::MenuLayoutChanged(name, layout))
            }))
        }
    });

    let mut module = TrayModule::with_factories(listener_spawner, command_factory);
    <TrayModule as Module<()>>::register(&mut module, &context, ()).expect("registration");

    // update() returns (), just verify it doesn't panic
    module.update(TrayMessage::MenuSelected("tray".into(), 42));

    let event = runtime
        .block_on(async {
            timeout(Duration::from_millis(100), async {
                loop {
                    if let Some(event) = receiver.try_recv().expect("bus read") {
                        break event;
                    }
                    yield_now().await;
                }
            })
            .await
        })
        .expect("event published");

    match event {
        BusEvent::Module(ModuleEvent::Tray(TrayMessage::Event(event))) => match *event {
            ServiceEvent::Update(TrayEvent::MenuLayoutChanged(ref name, _)) => {
                assert_eq!(name, "tray");
            }
            other => panic!("unexpected tray event: {other:?}")
        },
        other => panic!("unexpected bus event: {other:?}")
    }
}

#[test]
fn retains_default_listener_spawner() {
    let _module =
        TrayModule::with_factories(default_listener_spawner(), default_command_factory());
}
