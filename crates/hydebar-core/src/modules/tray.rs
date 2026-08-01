use std::{future::Future, pin::Pin, sync::Arc};

use tokio::{runtime::Handle, task::JoinHandle};

use crate::{
    ModuleEventSender,
    services::{
        ServiceEvent,
        tray::{TrayCommand, TrayService}
    }
};

#[derive(Debug, Clone)]
pub enum TrayMessage {
    Event(Box<ServiceEvent<TrayService>>),
    ToggleSubmenu(i32),
    MenuSelected(String, i32)
}

type ListenerSpawner =
    Arc<dyn Fn(ModuleEventSender<TrayMessage>, Handle) -> JoinHandle<()> + Send + Sync>;
type CommandFactory =
    Arc<dyn Fn(Option<&TrayService>, TrayCommand) -> Option<TrayCommandFuture> + Send + Sync>;
type TrayCommandFuture = Pin<Box<dyn Future<Output = ServiceEvent<TrayService>> + Send + 'static>>;

pub struct TrayModule {
    pub service:      Option<TrayService>,
    pub submenus:     Vec<i32>,
    sender:           Option<ModuleEventSender<TrayMessage>>,
    runtime:          Option<Handle>,
    listener_handles: Vec<JoinHandle<()>>,
    listener_spawner: ListenerSpawner,
    command_factory:  CommandFactory
}

impl std::fmt::Debug for TrayModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayModule")
            .field("service", &self.service)
            .field("submenus", &self.submenus)
            .field("sender", &self.sender)
            .field("runtime", &self.runtime)
            .field(
                "listener_handles",
                &format!("<{} handles>", self.listener_handles.len())
            )
            .field("listener_spawner", &"<function>")
            .field("command_factory", &"<function>")
            .finish()
    }
}

mod module {
    //! Module trait wiring for the tray.

    use std::sync::Arc;

    use iced::{Element, SurfaceId as Id};

    use super::super::{
        Module, ModuleError, OnModulePress,
        tray::{CommandFactory, ListenerSpawner, TrayMessage, TrayModule}
    };
    use crate::{ModuleContext, event_bus::ModuleEvent, services::tray::TrayService};

    impl<M> Module<M> for TrayModule
    where
        M: 'static + Clone
    {
        type ViewData<'a> = (Id, f32);
        type RegistrationData<'a> = ();

        fn register(
            &mut self,
            ctx: &ModuleContext,
            (): Self::RegistrationData<'_>
        ) -> Result<(), ModuleError> {
            self.abort_listener_handles();
            self.sender = Some(ctx.module_sender(ModuleEvent::Tray));
            self.runtime = Some(ctx.runtime_handle().clone());
            self.spawn_listener();

            Ok(())
        }

        /// Drops the status notifier listener once the tray leaves the bar.
        ///
        /// The listener owns the `StatusNotifierWatcher` name on D-Bus and
        /// receives every icon and menu change every tray application
        /// publishes. A bar without a tray area renders none of it, and
        /// holding the well known name would also keep other trays from
        /// taking over.
        fn deregister(&mut self) {
            self.abort_listener_handles();

            self.service = None;
            self.sender = None;
        }

        /// The bar strip is rendered by the GUI layer, like the battery: each
        /// icon toggles its own positioned menu, and those messages carry a
        /// [`ButtonUIRef`](crate::position_button::ButtonUIRef) that a view
        /// generic over its message type cannot construct.
        fn view(
            &self,
            (_id, _opacity): Self::ViewData<'_>
        ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
            None
        }

        fn subscription(&self) -> Option<iced::Subscription<M>> {
            None
        }
    }

    impl Default for TrayModule {
        fn default() -> Self {
            Self {
                service:          None,
                submenus:         Vec::new(),
                sender:           None,
                runtime:          None,
                listener_handles: Vec::new(),
                listener_spawner: default_listener_spawner(),
                command_factory:  default_command_factory()
            }
        }
    }

    impl Drop for TrayModule {
        fn drop(&mut self) {
            self.abort_listener_handles();
        }
    }

    pub(super) fn default_listener_spawner() -> ListenerSpawner {
        Arc::new(|sender, runtime| {
            runtime.spawn(async move {
                TrayService::start_listening(|event| {
                    let sender = sender.clone();
                    async move {
                        sender.send(TrayMessage::Event(Box::new(event)));
                    }
                })
                .await;
            })
        })
    }

    pub(super) fn default_command_factory() -> CommandFactory {
        Arc::new(|service, command| service.and_then(|svc| svc.prepare_command(command)))
    }
}
mod state {
    //! Listener wiring and message handling for the tray module.

    use std::sync::Arc;

    use log::{debug, error, warn};

    use super::super::tray::{TrayCommandFuture, TrayMessage, TrayModule};
    use crate::services::{ReadOnlyService, ServiceEvent, tray::TrayCommand};

    impl TrayModule {
        pub(super) fn abort_listener_handles(&mut self) {
            for handle in self.listener_handles.drain(..) {
                handle.abort();
            }
        }

        pub(super) fn spawn_listener(&mut self) {
            let Some(sender) = self.sender.clone() else {
                warn!("tray module missing event sender; skipping listener spawn");
                return;
            };
            let Some(runtime) = self.runtime.clone() else {
                warn!("tray module missing runtime handle; skipping listener spawn");
                return;
            };

            let spawner = Arc::clone(&self.listener_spawner);
            self.listener_handles.push(spawner(sender, runtime));
        }

        pub(super) fn dispatch_command(&self, command_future: TrayCommandFuture) {
            let Some(runtime) = self.runtime.clone() else {
                warn!("tray module missing runtime handle; skipping command dispatch");
                return;
            };
            let Some(sender) = self.sender.clone() else {
                warn!("tray module missing event sender; skipping command dispatch");
                return;
            };

            runtime.spawn(async move {
                let event = command_future.await;
                sender.send(TrayMessage::Event(Box::new(event)));
            });
        }

        pub fn update(&mut self, message: TrayMessage) {
            match message {
                TrayMessage::Event(event) => match *event {
                    ServiceEvent::Init(service) => {
                        self.service = Some(service);
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(service) = self.service.as_mut() {
                            service.update(data);
                        }
                    }
                    ServiceEvent::Error(()) => {
                        error!("Tray service error occurred");
                    }
                },
                TrayMessage::ToggleSubmenu(index) => {
                    if self.submenus.contains(&index) {
                        self.submenus.retain(|i| i != &index);
                    } else {
                        self.submenus.push(index);
                    }
                }
                TrayMessage::MenuSelected(name, id) => {
                    debug!("Tray menu click: {id}");

                    if let Some(command) = (self.command_factory)(
                        self.service.as_ref(),
                        TrayCommand::MenuSelected(name, id)
                    ) {
                        self.dispatch_command(command);
                    }
                }
            }
        }
    }
}
mod view {
    //! Rendering of tray icons and their menus.

    use iced::{
        Element, Length,
        widget::{Column, Row, button, row, rule, toggler}
    };

    use super::super::tray::{TrayMessage, TrayModule};
    use crate::{
        components::{
            icons::{IconTheme, Icons, icon},
            push_maybe::PushMaybe,
            scale,
            text::text
        },
        services::tray::dbus::{Layout, LayoutProps},
        style::ghost_button_style
    };

    impl TrayModule {
        #[must_use]
        pub fn menu_view(
            &self,
            name: &'_ str,
            opacity: f32,
            icons: &IconTheme
        ) -> Element<'_, TrayMessage> {
            self.service
                .as_ref()
                .and_then(|service| service.data.iter().find(|item| item.name == name))
                .map_or_else(
                    || Row::new().into(),
                    |item| {
                        Column::with_children(
                            item.menu
                                .2
                                .iter()
                                .map(|menu| self.menu_voice(name, menu, opacity, icons))
                        )
                        .spacing(scale::scaled(8.0))
                        .into()
                    }
                )
        }

        pub(super) fn menu_voice(
            &self,
            name: &str,
            layout: &Layout,
            opacity: f32,
            icons: &IconTheme
        ) -> Element<'_, TrayMessage> {
            match &layout.1 {
                LayoutProps {
                    label: Some(label),
                    toggle_type: Some(toggle_type),
                    toggle_state: Some(state),
                    ..
                } if toggle_type == "checkmark" => toggler(*state > 0)
                    .label(label.replace('_', ""))
                    .on_toggle({
                        let name = name.to_owned();
                        let id = layout.0;

                        move |_| TrayMessage::MenuSelected(name.clone(), id)
                    })
                    .width(Length::Fill)
                    .into(),
                LayoutProps {
                    children_display: Some(display),
                    label: Some(label),
                    ..
                } if display == "submenu" => {
                    let is_open = self.submenus.contains(&layout.0);
                    Column::new()
                        .push(
                            button(row!(
                                text(label.replace('_', "")).width(Length::Fill),
                                icon(
                                    icons,
                                    if is_open {
                                        Icons::MenuOpen
                                    } else {
                                        Icons::MenuClosed
                                    }
                                )
                            ))
                            .style(ghost_button_style(opacity))
                            .padding([scale::scaled(8.0), scale::scaled(8.0)])
                            .on_press(TrayMessage::ToggleSubmenu(layout.0))
                            .width(Length::Fill)
                        )
                        .push_maybe(if is_open {
                            Some(
                                Column::with_children(
                                    layout
                                        .2
                                        .iter()
                                        .map(|menu| self.menu_voice(name, menu, opacity, icons))
                                )
                                .padding(iced::Padding {
                                    top:    0.0,
                                    right:  0.0,
                                    bottom: 0.0,
                                    left:   16.0
                                })
                                .spacing(scale::scaled(4.0))
                            )
                        } else {
                            None
                        })
                        .into()
                }
                LayoutProps {
                    label: Some(label), ..
                } => button(text(label.replace('_', "")))
                    .style(ghost_button_style(opacity))
                    .on_press(TrayMessage::MenuSelected(name.to_owned(), layout.0))
                    .width(Length::Fill)
                    .padding([scale::scaled(8.0), scale::scaled(8.0)])
                    .into(),
                LayoutProps {
                    type_: Some(t), ..
                } if t == "separator" => rule::horizontal(1).into(),
                _ => Row::new().into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the tray module.

    use super::super::tray::{CommandFactory, ListenerSpawner, TrayModule};

    impl TrayModule {
        fn with_factories(
            listener_spawner: ListenerSpawner,
            command_factory: CommandFactory
        ) -> Self {
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
                        let mut signal = self.signal.lock().expect("cancellation lock");
                        if let Some(sender) = signal.take() {
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

        <TrayModule as Module<()>>::register(&mut module, &context, ())
            .expect("first registration");
        <TrayModule as Module<()>>::register(&mut module, &context, ())
            .expect("second registration");

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

        let listener_spawner: ListenerSpawner =
            Arc::new(|_, handle: Handle| handle.spawn(async {}));
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
                        if let Some(event) = receiver.try_recv() {
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
}
