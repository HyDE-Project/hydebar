//! Drawing of the clock entry: the time in its active format.

use iced::Element;

use super::{Clock, Message};
use crate::{
    components::{scale, text::text},
    config::ClockModuleConfig,
    menu::MenuType,
    modules::OnModulePress
};

impl Clock {
    /// The bar entry: the time in the format the user last chose.
    ///
    /// A clock declaring alternatives cycles them on the left button and moves
    /// the calendar to the right button, the way waybar binds its alternate
    /// format.
    ///
    /// Rendered by the module itself, so the bar layer holds no clock drawing
    /// of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        config: &ClockModuleConfig
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + Clone + From<Message>
    {
        let clock_text = if self.shown.current().is_empty() {
            text(self.data.format(self.active_format(config))).into()
        } else {
            self.shown.element(scale::base())
        };

        let on_press = if config.has_alternatives() {
            OnModulePress::Action(Box::new(M::from(Message::NextFormat)))
        } else {
            OnModulePress::ToggleMenu(MenuType::Calendar)
        };

        Some((clock_text, Some(on_press)))
    }
}
