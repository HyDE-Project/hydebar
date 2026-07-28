//! Module rendered from the output of a user provided command.
//!
//! The listener protocol is a superset of the Waybar custom module contract:
//! the process writes one JSON object per line to standard output and the bar
//! renders it.

mod data;
mod error;
mod listener;
mod poller;
mod state;
mod view;

use iced::Element;

pub use self::{
    data::CustomListenData,
    error::CustomCommandError,
    state::{Custom, CustomCommandService, Message}
};
use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext,
    components::icons::IconTheme,
    config::{Appearance, CustomModuleDef}
};

impl<M> Module<M> for Custom
where
    M: 'static + Clone
{
    type ViewData<'a> = (&'a CustomModuleDef, &'a Appearance, &'a IconTheme);
    type RegistrationData<'a> = Option<&'a CustomModuleDef>;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.start_listener(ctx, config)
    }

    fn view(
        &self,
        (config, appearance, icons): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        Some((view::render(self, config, appearance, icons), None))
    }
}

#[cfg(test)]
mod tests;
