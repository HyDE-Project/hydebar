//! Rendering of the settings window.
//!
//! Every page reads its values from the running configuration, so the window
//! shows the truth after a reload instead of a copy that drifted.

mod appearance;
mod metrics;
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

    /// Width the longest row of the current page needs.
    ///
    /// The window asks for exactly this much and no more: the screen only ever
    /// caps it, it never stretches it.
    #[must_use]
    pub fn content_width(&self, config: &Config) -> f32 {
        let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
        let tabs = Tab::ALL.into_iter().map(Tab::label).collect::<Vec<_>>();

        let header =
            metrics::text_width("Bar settings", font_size) + ROW_GAP_EM * font_size + font_size;
        let tab_row =
            metrics::button_row_width(tabs.into_iter(), font_size, ROW_GAP_EM * font_size);

        let page = match self.tab() {
            Tab::Appearance => appearance::desired_width(font_size),
            Tab::Modules => modules::desired_width(config, font_size)
        };

        header.max(tab_row).max(page) + metrics::ROW_SLACK_EM * font_size
    }
}
