use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::{HyprlandPort, HyprlandWindowEvent, HyprlandWindowInfo};
use iced::{Element, widget::text};
use log::error;
use tokio::{task::JoinHandle, time::sleep};
use tokio_stream::StreamExt;

use crate::{
    ModuleContext, ModuleEventSender,
    components::scale,
    config::{WindowTitleConfig, WindowTitleMode},
    event_bus::ModuleEvent,
    utils::truncate_text
};

const WINDOW_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

use super::{Module, ModuleError, OnModulePress};

/// Resolves the focused window off the async workers and publishes it.
///
/// The port call blocks on the compositor socket with retries; resolving it
/// here keeps both the runtime workers and the update thread free, and the
/// message arrives carrying its data instead of an order to go fetch some.
async fn publish_active_window(
    port: &Arc<dyn HyprlandPort>,
    sender: &crate::ModuleEventSender<Message>
) {
    let port = Arc::clone(port);

    match tokio::task::spawn_blocking(move || port.active_window()).await {
        Ok(Ok(window)) => {
            if let Err(err) = sender.try_send(Message::TitleChanged(window)) {
                error!("failed to publish window title update: {err}");
            }
        }
        Ok(Err(err)) => error!("failed to retrieve active window: {err}"),
        Err(err) => error!("active window task failed: {err}")
    }
}

fn get_window(port: &dyn HyprlandPort, config: &WindowTitleConfig) -> Option<String> {
    match port.active_window() {
        Ok(window) => window.map(|window| shown_field(window, config)),
        Err(err) => {
            error!("failed to retrieve active window: {err}");
            None
        }
    }
}

/// Field of the focused window the configured mode shows.
fn shown_field(window: HyprlandWindowInfo, config: &WindowTitleConfig) -> String {
    match config.mode {
        WindowTitleMode::Title => window.title,
        WindowTitleMode::Class => window.class
    }
}

/// The title as the bar draws it.
///
/// Whole while the module is attended, shortened to the configured length
/// otherwise: a title that had to be cut is the one the user leans in to read,
/// so looking at the module is taken as asking for the rest of it.
fn shown_title(value: &str, config: &WindowTitleConfig, attended: bool) -> String {
    if attended {
        value.to_owned()
    } else {
        truncate_text(value, config.truncate_title_after_length)
    }
}

/// Title of the focused window, as the compositor reports it.
///
/// The whole title is kept rather than the shortened one the bar draws: a
/// module the user is looking at shows what it has in full, and a title
/// shortened on the way in could never be restored.
pub struct WindowTitle {
    hyprland: Arc<dyn HyprlandPort>,
    value:    Option<String>,
    sender:   Option<ModuleEventSender<Message>>,
    task:     Option<JoinHandle<()>>
}

impl std::fmt::Debug for WindowTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowTitle")
            .field("hyprland", &"<HyprlandPort>")
            .field("value", &self.value)
            .field("sender", &self.sender)
            .field("task", &self.task)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(Option<HyprlandWindowInfo>)
}

impl WindowTitle {
    pub fn new(hyprland: Arc<dyn HyprlandPort>, config: &WindowTitleConfig) -> Self {
        let init = get_window(hyprland.as_ref(), config);

        Self {
            hyprland,
            value: init,
            sender: None,
            task: None
        }
    }
}

impl WindowTitle {
    pub fn update(&mut self, message: Message, config: &WindowTitleConfig) {
        match message {
            Message::TitleChanged(window) => {
                self.value = window.map(|window| shown_field(window, config));
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn current_value(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

impl<M> Module<M> for WindowTitle
where
    M: 'static + Clone
{
    type ViewData<'a> = (&'a WindowTitleConfig, bool);
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::WindowTitle));

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(async move {
                loop {
                    match hyprland.window_events() {
                        Ok(mut stream) => {
                            while let Some(event) = stream.next().await {
                                match event {
                                    Ok(
                                        HyprlandWindowEvent::ActiveWindowChanged
                                        | HyprlandWindowEvent::WindowTitleChanged
                                        | HyprlandWindowEvent::WindowClosed
                                        | HyprlandWindowEvent::WorkspaceFocusChanged
                                    ) => {
                                        publish_active_window(&hyprland, &sender).await;
                                    }
                                    Err(err) => {
                                        error!("window event stream error: {err}");
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("failed to start window event stream: {err}");
                        }
                    }

                    sleep(WINDOW_EVENT_RETRY_DELAY).await;
                }
            }));
        }

        Ok(())
    }

    /// Drops the compositor event stream once the title leaves the bar.
    ///
    /// Focus changes are among the most frequent events a session produces, so
    /// a listener nobody renders is the most expensive one to leave running.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }

    /// Draws the title, in full while the module is attended.
    ///
    /// A title long enough to be shortened is exactly the one the user leans
    /// in to read, so looking at the module is taken as asking for the rest of
    /// it.
    fn view(
        &self,
        (config, attended): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        self.value.as_ref().map(|value| {
            let shown = shown_title(value, config, attended);

            (
                text(shown)
                    .size(scale::scaled(12.0))
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .into(),
                None
            )
        })
    }

    // No iced subscription required; updates are dispatched via the module event
    // sender.
}

#[cfg(test)]
mod tests {
    use hydebar_proto::config::{WindowTitleConfig, WindowTitleMode};

    use super::*;
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn initializes_title_from_port() {
        let port = Arc::new(MockHyprlandPort::with_active_window("Demo", "Class"));
        let port_trait: Arc<dyn HyprlandPort> = port;
        let config = WindowTitleConfig {
            mode: WindowTitleMode::Title,
            ..Default::default()
        };

        let module = WindowTitle::new(port_trait, &config);

        assert_eq!(module.current_value(), Some("Demo"));
    }

    #[test]
    fn update_handles_absent_window() {
        let port = Arc::new(MockHyprlandPort::default());
        *port
            .active_window
            .lock()
            .expect("active window lock poisoned") = None;
        let port_trait: Arc<dyn HyprlandPort> = port;
        let config = WindowTitleConfig::default();

        let mut module = WindowTitle::new(port_trait, &config);
        module.update(Message::TitleChanged(None), &config);

        assert_eq!(module.current_value(), None);
    }

    #[test]
    fn a_long_title_is_shortened_until_the_module_is_attended() {
        let config = WindowTitleConfig {
            truncate_title_after_length: 10,
            ..Default::default()
        };
        let title = "a window with a very long title indeed";

        assert_ne!(shown_title(title, &config, false), title);
        assert_eq!(shown_title(title, &config, true), title);
    }

    #[test]
    fn a_short_title_reads_the_same_either_way() {
        let config = WindowTitleConfig {
            truncate_title_after_length: 150,
            ..Default::default()
        };

        assert_eq!(shown_title("short", &config, false), "short");
        assert_eq!(shown_title("short", &config, true), "short");
    }

    #[test]
    fn the_whole_title_survives_the_update() {
        let port = Arc::new(MockHyprlandPort::with_active_window(
            "a window with a very long title indeed",
            "Class"
        ));
        let port_trait: Arc<dyn HyprlandPort> = port;
        let config = WindowTitleConfig {
            truncate_title_after_length: 10,
            ..Default::default()
        };

        let mut module = WindowTitle::new(port_trait, &config);
        module.update(
            Message::TitleChanged(Some(HyprlandWindowInfo {
                title: "a window with a very long title indeed".to_owned(),
                class: "Class".to_owned()
            })),
            &config
        );

        assert_eq!(
            module.current_value(),
            Some("a window with a very long title indeed"),
            "a title shortened on the way in could never be shown in full"
        );
    }
}
