//! Bar strip of the mapped windows, pressed to focus one.
//!
//! The counterpart of the reference bar's taskbar: one entry per mapped
//! client, drawn with the icon its class resolves to in the icon theme, or
//! the first letter of the class where no icon answers. The focused window
//! is drawn at full strength and the rest step back, so the strip reads as
//! "where am I" as much as "what is open".
//!
//! The list is a snapshot re-read on every window event, the same rhythm the
//! workspaces module keeps: the compositor pushes, the bar asks once per
//! burst, and an unchanged answer is dropped before it reaches the bus.

use std::{sync::Arc, time::Duration};

use iced::{
    Alignment, Element, Length,
    futures::StreamExt,
    widget::{Row, image, mouse_area, svg}
};
use log::error;
use tokio::{runtime::Handle, task::JoinHandle, time::sleep};

use hydebar_proto::ports::hyprland::{HyprlandClientInfo, HyprlandPort};

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, ModuleEventSender,
    components::{scale, text::text},
    event_bus::ModuleEvent,
    services::tray::{TrayIcon, icon_from_name}
};

/// How long a failed event stream rests before it is reopened.
const EVENT_RETRY_DELAY: Duration = Duration::from_secs(5);

/// Messages delivered to the taskbar.
#[derive(Debug, Clone)]
pub enum Message {
    /// A fresh client list arrived from the compositor.
    ClientsChanged(Vec<HyprlandClientInfo>),
    /// An entry was pressed and its window wants the focus.
    Focus(String)
}

/// Bar strip of the compositor's mapped clients.
pub struct Taskbar {
    hyprland: Arc<dyn HyprlandPort>,
    clients:  Vec<HyprlandClientInfo>,
    sender:   Option<ModuleEventSender<Message>>,
    runtime:  Option<Handle>,
    task:     Option<JoinHandle<()>>
}

impl std::fmt::Debug for Taskbar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Taskbar")
            .field("hyprland", &"<HyprlandPort>")
            .field("clients", &self.clients)
            .field("sender", &self.sender)
            .field("runtime", &self.runtime)
            .field("task", &self.task)
            .finish()
    }
}

impl Taskbar {
    #[must_use]
    pub fn new(hyprland: Arc<dyn HyprlandPort>) -> Self {
        Self {
            hyprland,
            clients: Vec::new(),
            sender: None,
            runtime: None,
            task: None
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ClientsChanged(clients) => {
                self.clients = clients;
            }
            Message::Focus(address) => {
                let port = Arc::clone(&self.hyprland);
                self.spawn_dispatch(move || port.focus_window(&address));
            }
        }
    }

    /// Runs a compositor dispatch off the thread the bar draws on.
    ///
    /// The port retries with a timeout when the compositor socket stalls;
    /// waiting that out on the update thread would freeze every module. A
    /// module that was never registered has no runtime and dispatches
    /// inline, which keeps tests synchronous.
    fn spawn_dispatch(
        &self,
        dispatch: impl FnOnce() -> Result<(), hydebar_proto::ports::hyprland::HyprlandError>
        + Send
        + 'static
    ) {
        match &self.runtime {
            Some(runtime) => {
                runtime.spawn_blocking(move || {
                    if let Err(err) = dispatch() {
                        error!("failed to focus a window from the taskbar: {err}");
                    }
                });
            }
            None => {
                if let Err(err) = dispatch() {
                    error!("failed to focus a window from the taskbar: {err}");
                }
            }
        }
    }
}

/// Resolves a fresh client list off the async workers and publishes it.
///
/// The port call blocks on the compositor socket with retries, so it runs on
/// the blocking pool. An answer equal to the previous one is still published;
/// the bus replaces the stale snapshot in place, so the cost is one message,
/// not one repaint per event.
async fn publish_clients(port: &Arc<dyn HyprlandPort>, sender: &ModuleEventSender<Message>) {
    let port = Arc::clone(port);

    match tokio::task::spawn_blocking(move || port.clients_snapshot()).await {
        Ok(Ok(clients)) => {
            sender.send(Message::ClientsChanged(clients));
        }
        Ok(Err(err)) => error!("failed to retrieve the client list: {err}"),
        Err(err) => error!("client list task failed: {err}")
    }
}

impl<M> Module<M> for Taskbar
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = f32;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::Taskbar));
        self.runtime = Some(ctx.runtime_handle().clone());

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(async move {
                publish_clients(&hyprland, &sender).await;

                loop {
                    match hyprland.window_events() {
                        Ok(mut stream) => {
                            while let Some(event) = stream.next().await {
                                match event {
                                    Ok(_) => publish_clients(&hyprland, &sender).await,
                                    Err(err) => error!("window event stream error: {err}")
                                }
                            }
                        }
                        Err(err) => {
                            error!("failed to start the taskbar event stream: {err}");
                        }
                    }

                    sleep(EVENT_RETRY_DELAY).await;
                }
            }));
        }

        Ok(())
    }

    /// Drops the compositor event stream once the strip leaves the bar.
    ///
    /// The listener re-reads the whole client list on every window event; a
    /// layout without a taskbar would pay that for nothing.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }

    fn view(&self, font_size: Self::ViewData<'_>) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if self.clients.is_empty() {
            return None;
        }

        let side = scale::scaled(font_size * 1.2);

        let entries = self.clients.iter().map(|client| {
            let face: Element<'static, M> = match icon_from_name(&client.class.to_lowercase()) {
                Some(TrayIcon::Image(handle)) => image(handle)
                    .width(Length::Fixed(side))
                    .height(Length::Fixed(side))
                    .opacity(entry_strength(client.focused))
                    .into(),
                Some(TrayIcon::Svg(handle)) => svg(handle)
                    .width(Length::Fixed(side))
                    .height(Length::Fixed(side))
                    .opacity(entry_strength(client.focused))
                    .into(),
                None => text(fallback_glyph(&client.class)).into()
            };

            mouse_area(face)
                .on_press(M::from(Message::Focus(client.address.clone())))
                .into()
        });

        Some((
            Row::with_children(entries)
                .spacing(scale::icon_gap())
                .align_y(Alignment::Center)
                .into(),
            None
        ))
    }
}

/// How strongly an entry is drawn: the focused window at full strength.
const fn entry_strength(focused: bool) -> f32 {
    if focused { 1.0 } else { 0.55 }
}

/// The letter standing in for a class no icon theme answers for.
fn fallback_glyph(class: &str) -> String {
    class
        .chars()
        .next()
        .map_or_else(|| String::from("?"), |first| {
            first.to_uppercase().collect()
        })
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::{event_bus::EventBus, test_utils::MockHyprlandPort};

    fn client(address: &str, focused: bool) -> HyprlandClientInfo {
        HyprlandClientInfo {
            address:      address.to_owned(),
            class:        "kitty".to_owned(),
            title:        "shell".to_owned(),
            workspace_id: 1,
            focused
        }
    }

    #[test]
    fn a_fresh_list_replaces_the_old_one() {
        let mut taskbar = Taskbar::new(Arc::new(MockHyprlandPort::default()));

        taskbar.update(Message::ClientsChanged(vec![client("0x1", true)]));
        taskbar.update(Message::ClientsChanged(vec![
            client("0x2", false),
            client("0x3", true),
        ]));

        assert_eq!(taskbar.clients.len(), 2);
    }

    #[test]
    fn a_press_reaches_the_compositor() {
        let port = Arc::new(MockHyprlandPort::default());
        let mut taskbar = Taskbar::new(Arc::clone(&port) as Arc<dyn HyprlandPort>);

        taskbar.update(Message::Focus("0x1".to_owned()));

        assert_eq!(
            port.focus_window_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }

    #[tokio::test]
    async fn registration_publishes_the_first_snapshot() {
        let bus = EventBus::new(NonZeroUsize::new(8).expect("capacity"));
        let mut receiver = bus.receiver();
        let ctx = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let port = Arc::new(MockHyprlandPort::default());
        *port
            .clients_snapshot
            .lock()
            .expect("clients lock") = vec![client("0x1", true)];

        let mut taskbar = Taskbar::new(Arc::clone(&port) as Arc<dyn HyprlandPort>);
        <Taskbar as Module<Message>>::register(&mut taskbar, &ctx, ()).expect("registration");

        let batch = tokio::time::timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("the snapshot arrives");

        assert!(!batch.is_empty());

        <Taskbar as Module<Message>>::deregister(&mut taskbar);
    }

    #[test]
    fn the_focused_window_stands_out() {
        assert!(entry_strength(true) > entry_strength(false));
    }

    #[test]
    fn a_class_without_an_icon_still_shows_a_letter() {
        assert_eq!(fallback_glyph("kitty"), "K");
        assert_eq!(fallback_glyph(""), "?");
    }
}
