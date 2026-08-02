//! Module trait wiring for the window title.
//!
//! No iced subscription is involved: the listener publishes focused-window
//! changes onto the event bus and the bar folds them in through
//! [`WindowTitle::update`](super::WindowTitle::update).

use std::sync::Arc;

use iced::{Element, widget::text};

use super::{WindowTitle, listener, state::shown_title};
use crate::{
    ModuleContext,
    components::scale,
    config::WindowTitleConfig,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError, OnModulePress}
};

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
            self.task = Some(ctx.runtime_handle().spawn(listener::run(hyprland, sender)));
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
            let shown = if attended {
                value.clone()
            } else {
                self.shortened
                    .clone()
                    .unwrap_or_else(|| shown_title(value, config, attended))
            };

            (
                text(shown)
                    .size(scale::scaled(12.0))
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .into(),
                None
            )
        })
    }
}
