//! Rendering of the settings window.
//!
//! Every page reads its values from the running configuration, so the window
//! shows the truth after a reload instead of a copy that drifted.

mod appearance;
mod modules;
mod widgets;

use iced::{
    Alignment, Element, Length,
    widget::{Column, Row, text}
};

use self::widgets::{ROW_GAP_EM, choice_button};
use super::{Message, Settings, Tab};
use crate::{
    components::icons::{IconTheme, Icons, icon},
    config::{Config, DEFAULT_FONT_SIZE}
};

/// Gap between the header, the tabs and the page, in multiples of the text
/// size.
const WINDOW_GAP_EM: f32 = 1.0;

impl Settings {
    /// Renders the settings window against the running `config`.
    ///
    /// `content_width` is the room the window body may spend, so pages that
    /// list many entries can wrap instead of overflowing.
    pub fn menu_view<'a>(
        &self,
        config: &'a Config,
        opacity: f32,
        icons: &IconTheme,
        content_width: f32
    ) -> Element<'a, Message> {
        let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
        let active = self.tab();

        let header = Row::new()
            .push(icon(icons, Icons::Settings))
            .push(text("Bar settings").size(font_size).width(Length::Fill))
            .spacing(ROW_GAP_EM * font_size)
            .align_y(Alignment::Center);

        let mut tabs = Row::new().spacing(ROW_GAP_EM * font_size);

        for tab in Tab::ALL {
            tabs = tabs.push(choice_button(
                tab.label(),
                Message::SelectTab(tab),
                tab == active,
                font_size,
                opacity
            ));
        }

        let page = match active {
            Tab::Appearance => appearance::view(config, opacity),
            Tab::Modules => modules::view(config, opacity, font_size, content_width)
        };

        Column::new()
            .push(header)
            .push(tabs)
            .push(page)
            .width(Length::Fill)
            .spacing(WINDOW_GAP_EM * font_size)
            .into()
    }
}
