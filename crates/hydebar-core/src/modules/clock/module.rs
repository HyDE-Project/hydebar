//! Module trait wiring for the clock entry.

use iced::Element;

use super::{Clock, Message};
use crate::{
    ModuleContext,
    components::text::text,
    config::ClockModuleConfig,
    menu::MenuType,
    modules::{Module, ModuleError, OnModulePress}
};

impl<M> Module<M> for Clock
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = &'a ClockModuleConfig;
    type RegistrationData<'a> = &'a ClockModuleConfig;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.register(ctx, config);
        Ok(())
    }

    /// Stops the tick loop once the clock leaves the bar.
    ///
    /// A tick repaints every surface the bar owns, which is pure waste when no
    /// section renders the time any more.
    fn deregister(&mut self) {
        self.stop();
    }

    /// Renders the clock in its active format.
    ///
    /// A clock declaring alternatives cycles them on the left button and moves
    /// the calendar to the right button, the way waybar binds `format-alt`.
    fn view(
        &self,
        config: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        let clock_text = if self.shown.current().is_empty() {
            text(self.data.format(self.active_format(config))).into()
        } else {
            self.shown.element(crate::components::scale::base())
        };
        let on_press = if config.has_alternatives() {
            OnModulePress::Action(Box::new(M::from(Message::NextFormat)))
        } else {
            OnModulePress::ToggleMenu(MenuType::Calendar)
        };

        Some((clock_text, Some(on_press)))
    }
}
