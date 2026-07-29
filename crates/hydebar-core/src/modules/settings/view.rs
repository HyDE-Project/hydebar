//! Rendering of the settings window.
//!
//! Every page reads its values from the running configuration, so the window
//! shows the truth after a reload instead of a copy that drifted.

mod appearance;
mod hyde;
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
    /// `magnification` is the factor the bar is drawn at, so the pages can show
    /// the sizes as they are written in the file rather than as they render.
    pub fn menu_view<'a>(
        &self,
        config: &'a Config,
        opacity: f32,
        icons: &IconTheme,
        magnification: f32
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
            Tab::Appearance => appearance::view(config, opacity, magnification),
            Tab::Modules => modules::view(
                config,
                opacity,
                font_size,
                self.section(),
                self.selected(),
                self.content_width(config) - metrics::ROW_SLACK_EM * font_size
            ),
            Tab::Hyde => hyde::view(
                self.hyde(),
                opacity,
                font_size,
                self.content_width(config) - metrics::ROW_SLACK_EM * font_size
            )
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
            Tab::Modules => modules::desired_width(config, font_size, self.section()),
            Tab::Hyde => hyde::desired_width(self.hyde(), font_size)
        };

        header.max(tab_row).max(page) + metrics::ROW_SLACK_EM * font_size
    }

    /// Height the current page needs.
    ///
    /// Measured rather than guessed so the window can be capped to the screen
    /// and scroll the rest: a page taller than the screen would otherwise have
    /// its last rows cut off by the edge.
    #[must_use]
    pub fn content_height(&self, config: &Config) -> f32 {
        let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);

        let header = metrics::ROW_HEIGHT_EM * font_size;
        let tabs = metrics::ROW_HEIGHT_EM * font_size;
        let page = match self.tab() {
            Tab::Appearance => appearance::desired_height(font_size),
            Tab::Modules => modules::desired_height(config, font_size, self.section()),
            Tab::Hyde => hyde::desired_height(
                self.hyde(),
                font_size,
                self.content_width(config) - metrics::ROW_SLACK_EM * font_size
            )
        };

        header + tabs + page + WINDOW_GAP_EM * font_size * 3.0
    }
}
