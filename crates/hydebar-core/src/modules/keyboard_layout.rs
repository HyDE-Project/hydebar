use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::{HyprlandKeyboardEvent, HyprlandKeyboardState, HyprlandPort};
use iced::{Element, widget::text};
use log::error;
use tokio::{task::JoinHandle, time::sleep};
use tokio_stream::StreamExt;

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, ModuleEventSender, config::KeyboardLayoutModuleConfig, event_bus::ModuleEvent
};

const KEYBOARD_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

pub struct KeyboardLayout {
    hyprland:        Arc<dyn HyprlandPort>,
    multiple_layout: bool,
    active:          String,
    sender:          Option<ModuleEventSender<Message>>,
    task:            Option<JoinHandle<()>>,
    shown:           crate::components::crossfade::Crossfade
}

impl std::fmt::Debug for KeyboardLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyboardLayout")
            .field("hyprland", &"<HyprlandPort>")
            .field("multiple_layout", &self.multiple_layout)
            .field("active", &self.active)
            .field("sender", &self.sender)
            .field("task", &self.task.as_ref().map(|_| "<JoinHandle>"))
            .field("shown", &self.shown)
            .finish()
    }
}

impl Clone for KeyboardLayout {
    fn clone(&self) -> Self {
        Self {
            hyprland:        Arc::clone(&self.hyprland),
            multiple_layout: self.multiple_layout,
            active:          self.active.clone(),
            sender:          self.sender.clone(),
            task:            None, // JoinHandle can't be cloned
            shown:           self.shown.clone()
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    LayoutConfigChanged(bool),
    ActiveLayoutChanged(String),
    ChangeLayout
}

impl KeyboardLayout {
    pub fn new(hyprland: Arc<dyn HyprlandPort>) -> Self {
        let HyprlandKeyboardState {
            active_layout,
            has_multiple_layouts,
            ..
        } = hyprland
            .keyboard_state()
            .unwrap_or_else(|_| HyprlandKeyboardState {
                active_layout:        "unknown".to_string(),
                has_multiple_layouts: false,
                active_submap:        None
            });

        Self {
            hyprland,
            multiple_layout: has_multiple_layouts,
            active: active_layout,
            sender: None,
            task: None,
            shown: crate::components::crossfade::Crossfade::default()
        }
    }

    /// `animated` decides whether the shown label dissolves into its
    /// replacement or swaps outright.
    pub fn update(
        &mut self,
        message: Message,
        config: &KeyboardLayoutModuleConfig,
        animated: bool
    ) {
        match message {
            Message::ActiveLayoutChanged(layout) => {
                self.active = layout;
            }
            Message::LayoutConfigChanged(layout_flag) => self.multiple_layout = layout_flag,
            Message::ChangeLayout => {
                if let Err(err) = self.hyprland.switch_keyboard_layout() {
                    error!("failed to switch keyboard layout: {err}");
                }
            }
        }

        let label = match config.labels.get(&self.active) {
            Some(value) => value.clone(),
            None => self.active.clone()
        };
        self.shown.set(label, animated);
    }

    /// Advances the dissolve of the shown label.
    pub fn tick_fade(&mut self, elapsed: Duration) -> bool {
        self.shown.advance(elapsed)
    }

    /// Whether the shown label is still dissolving.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        self.shown.is_animating()
    }

    /// Layout currently in force, as the compositor names it.
    #[must_use]
    pub fn active_layout(&self) -> &str {
        &self.active
    }

    #[cfg(test)]
    pub(crate) const fn has_multiple_layouts(&self) -> bool {
        self.multiple_layout
    }
}

impl<M> Module<M> for KeyboardLayout
where
    M: 'static + Clone
{
    type ViewData<'a> = &'a KeyboardLayoutModuleConfig;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::KeyboardLayout));

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
                                    Ok(HyprlandKeyboardEvent::LayoutChanged(layout)) => {
                                        sender.send(Message::ActiveLayoutChanged(layout));
                                    }
                                    Ok(HyprlandKeyboardEvent::LayoutConfigurationChanged(
                                        flag
                                    )) => {
                                        sender.send(Message::LayoutConfigChanged(flag));
                                    }
                                    Ok(HyprlandKeyboardEvent::SubmapChanged(_)) => {}
                                    Err(err) => {
                                        error!("keyboard event stream error: {err}");
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("failed to start keyboard event stream: {err}");
                        }
                    }

                    sleep(KEYBOARD_EVENT_RETRY_DELAY).await;
                }
            }));
        }

        Ok(())
    }

    /// Drops the keyboard event stream once the layout indicator leaves the
    /// bar.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }

    fn view(
        &self,
        config: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if self.multiple_layout {
            let label = if self.shown.current().is_empty() {
                let active = config
                    .labels
                    .get(&self.active)
                    .map_or_else(|| self.active.clone(), Clone::clone);

                text(active).into()
            } else {
                self.shown.element(crate::components::scale::base())
            };

            Some((
                label, None // Action handled in GUI layer
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn initializes_from_keyboard_state() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port;

        let module = KeyboardLayout::new(port_trait);

        assert_eq!(module.active_layout(), "us");
        assert!(module.has_multiple_layouts());
    }

    #[test]
    fn change_layout_invokes_port_command() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let mut module = KeyboardLayout::new(port_trait);

        module.update(
            Message::ChangeLayout,
            &hydebar_proto::config::KeyboardLayoutModuleConfig::default(),
            false
        );

        assert_eq!(port.switch_layout_calls(), 1);
    }
}
