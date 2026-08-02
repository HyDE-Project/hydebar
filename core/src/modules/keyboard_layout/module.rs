//! Module trait wiring for the keyboard layout indicator.
//!
//! Background updates are delivered via the shared module event sender: the
//! listener publishes layout changes onto the event bus and the bar folds
//! them in through
//! [`KeyboardLayout::update`](super::KeyboardLayout::update).

use std::sync::Arc;

use iced::{Element, widget::text};

use super::{KeyboardLayout, listener};
use crate::{
    ModuleContext,
    config::KeyboardLayoutModuleConfig,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError, OnModulePress}
};

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
            self.task = Some(ctx.runtime_handle().spawn(listener::run(hyprland, sender)));
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

    /// Draws the active layout's label when more than one layout is
    /// configured. The press action is handled in the GUI layer.
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

            Some((label, None))
        } else {
            None
        }
    }
}
