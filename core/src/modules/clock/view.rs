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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::menu::MenuType;

    fn config(alternatives: &[&str]) -> ClockModuleConfig {
        ClockModuleConfig {
            format:       "%H:%M".to_owned(),
            format_alt:   alternatives.iter().map(ToString::to_string).collect(),
            show_weather: false
        }
    }

    #[test]
    fn a_clock_is_always_on_the_strip() {
        let clock = Clock::new();

        assert!(clock.bar_view::<Message>(&config(&[])).is_some());
    }

    #[test]
    fn a_clock_without_alternatives_opens_the_calendar() {
        let clock = Clock::new();

        let (_, press) = clock
            .bar_view::<Message>(&config(&[]))
            .expect("the clock draws");

        assert!(matches!(
            press,
            Some(OnModulePress::ToggleMenu(MenuType::Calendar))
        ));
    }

    #[test]
    fn a_clock_with_alternatives_walks_them_instead() {
        let clock = Clock::new();

        let (_, press) = clock
            .bar_view::<Message>(&config(&["%A"]))
            .expect("the clock draws");

        assert!(matches!(press, Some(OnModulePress::Action(_))));
    }
}
