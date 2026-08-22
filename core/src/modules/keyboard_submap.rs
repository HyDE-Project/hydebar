use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::{HyprlandKeyboardEvent, HyprlandKeyboardState, HyprlandPort};
use iced::{Element, widget::text};
use log::error;
use tokio::{task::JoinHandle, time::sleep};
use tokio_stream::StreamExt;

use super::{Module, ModuleError, OnModulePress};
use crate::{ModuleContext, ModuleEventSender, event_bus::ModuleEvent};

pub struct KeyboardSubmap {
    hyprland: Arc<dyn HyprlandPort>,
    submap:   String,
    sender:   Option<ModuleEventSender<Message>>,
    task:     Option<JoinHandle<()>>,
    shown:    crate::components::crossfade::Crossfade
}

impl std::fmt::Debug for KeyboardSubmap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyboardSubmap")
            .field("hyprland", &"<HyprlandPort>")
            .field("submap", &self.submap)
            .field("sender", &self.sender)
            .field("task", &self.task)
            .field("shown", &self.shown)
            .finish()
    }
}

const SUBMAP_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

impl KeyboardSubmap {
    /// The submap the compositor is in, empty while it is in none.
    #[must_use]
    pub fn active(&self) -> &str {
        &self.submap
    }

    pub fn new(hyprland: Arc<dyn HyprlandPort>) -> Self {
        let initial_submap = hyprland
            .keyboard_state()
            .unwrap_or(HyprlandKeyboardState {
                active_layout:        String::new(),
                has_multiple_layouts: false,
                active_submap:        None
            })
            .active_submap
            .unwrap_or_default();

        Self {
            hyprland,
            submap: initial_submap,
            sender: None,
            task: None,
            shown: crate::components::crossfade::Crossfade::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SubmapChanged(String)
}

impl KeyboardSubmap {
    /// `animated` decides whether the shown submap dissolves into its
    /// replacement or swaps outright.
    pub fn update(&mut self, message: Message, animated: bool) {
        match message {
            Message::SubmapChanged(submap) => {
                self.submap = submap;
            }
        }

        self.shown.set(self.submap.clone(), animated);
    }

    /// Advances the dissolve of the shown submap.
    pub fn tick_fade(&mut self, elapsed: Duration) -> bool {
        self.shown.advance(elapsed)
    }

    /// Whether the shown submap is still dissolving.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        self.shown.is_animating()
    }

    #[cfg(test)]
    pub(crate) fn submap(&self) -> &str {
        &self.submap
    }
}

impl<M> Module<M> for KeyboardSubmap
where
    M: 'static + Clone
{
    type ViewData<'a> = ();
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::KeyboardSubmap));

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(async move {
                loop {
                    match hyprland.keyboard_events() {
                        Ok(mut stream) => {
                            while let Some(event) = stream.next().await {
                                match event {
                                    Ok(HyprlandKeyboardEvent::SubmapChanged(submap)) => {
                                        let payload = submap.unwrap_or_default();
                                        sender.send(Message::SubmapChanged(payload));
                                    }
                                    Ok(_) => {}
                                    Err(err) => {
                                        error!("keyboard submap stream error: {err}");
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("failed to start keyboard submap stream: {err}");
                        }
                    }

                    sleep(SUBMAP_EVENT_RETRY_DELAY).await;
                }
            }));
        }

        Ok(())
    }

    /// Drops the submap event stream once the indicator leaves the bar.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }

    fn view(
        &self,
        (): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if self.submap.is_empty() {
            None
        } else if self.shown.current().is_empty() {
            Some((text(self.submap.clone()).into(), None))
        } else {
            Some((self.shown.element(crate::components::scale::base()), None))
        }
    }

    // No iced subscription required; updates are dispatched via the module event
    // sender.
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn initializes_with_port_submap() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port;

        let module = KeyboardSubmap::new(port_trait);

        assert_eq!(module.submap(), "resize");
    }

    #[test]
    fn update_replaces_submap_value() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port;
        let mut module = KeyboardSubmap::new(port_trait);

        module.update(Message::SubmapChanged("launch".into()), false);

        assert_eq!(module.submap(), "launch");
    }
}
