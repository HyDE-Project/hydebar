//! Rendering of the settings window.
//!
//! Every page reads its values from the running configuration, so the
//! window shows the truth after a reload instead of a copy that
//! drifted.

mod appearance;
mod modules;

use iced::{
    Alignment, Element, Length,
    widget::{Column, Row}
};

use super::{Message, Settings, Tab};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        page::{metrics, style, widgets::choice_button},
        text::text
    },
    config::{Config, DEFAULT_FONT_SIZE}
};

/// Title drawn beside the icon at the top of the window.
///
/// Named once so the header the window measures is the header it draws: a
/// title changed in one place only would size the window for the other
/// one.
const TITLE: &str = "Bar settings";

impl Settings {
    /// Renders the settings window against the running `config`.
    ///
    /// `magnification` is the factor the bar is drawn at, so the pages can
    /// show the sizes as they are written in the file rather than
    /// as they render.
    #[must_use]
    pub fn menu_view<'a>(
        &self,
        config: &'a Config,
        opacity: f32,
        icons: &IconTheme,
        magnification: f32,
        page_width: f32
    ) -> Element<'a, Message> {
        let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
        let active = self.tab();

        let header = Row::new()
            .push(icon(icons, Icons::Settings))
            .push(text(TITLE).size(font_size).width(Length::Fill))
            .spacing(style::row_gap(font_size))
            .align_y(Alignment::Center);

        let mut tabs = Row::new().spacing(style::group_gap(font_size));

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
                page_width
            )
        };

        Column::new()
            .push(header)
            .push(tabs)
            .push(page)
            .width(Length::Fill)
            .spacing(style::window_gap(font_size))
            .into()
    }

    /// Width a page is actually given to draw into, slack excluded.
    ///
    /// The grid that wraps — the module catalogue — has to wrap against the
    /// room it gets rather than against the room the window asks for, or it
    /// would fit one chip too many and run past the edge.
    /// Width the longest row of the current page needs.
    ///
    /// The window asks for exactly this much and no more: the screen only
    /// ever caps it, it never stretches it.
    #[must_use]
    pub fn content_width(&self, config: &Config) -> f32 {
        let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
        let tabs = Tab::ALL.into_iter().map(Tab::label).collect::<Vec<_>>();

        let header = metrics::text_width(TITLE, font_size)
            + style::row_gap(font_size)
            + style::icon_width(font_size);
        let tab_row = metrics::button_row_width(
            tabs,
            style::control_size(font_size),
            style::group_gap(font_size)
        );

        let page = match self.tab() {
            Tab::Appearance => appearance::desired_width(font_size),
            Tab::Modules => modules::desired_width(config, font_size, self.section())
        };

        metrics::ROW_SLACK_EM.mul_add(font_size, header.max(tab_row).max(page))
    }

    /// Height the current page needs.
    ///
    /// Measured rather than guessed so the window can be capped to the
    /// screen and scroll the rest: a page taller than the screen
    /// would otherwise have its last rows cut off by the edge.
    /// The three window lengths, with the content walked exactly once.
    #[must_use]
    pub fn window_metrics(&self, config: &Config) -> crate::menu::MenuMetrics {
        let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
        let width = self.content_width(config);

        crate::menu::MenuMetrics {
            width,
            page_width: metrics::ROW_SLACK_EM.mul_add(-font_size, width),
            height: self.content_height(config)
        }
    }

    #[must_use]
    pub fn content_height(&self, config: &Config) -> f32 {
        let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);

        let header = style::row_height(font_size);
        let tabs = style::row_height(font_size);
        let page = match self.tab() {
            Tab::Appearance => appearance::desired_height(
                font_size,
                config.appearance.auto_scale,
                config.updates.is_some()
            ),
            Tab::Modules => modules::desired_height(config, font_size, self.section())
        };

        style::window_gap(font_size).mul_add(style::WINDOW_GAP_COUNT, header + tabs + page)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, clippy::suboptimal_flops)]

    use super::*;
    use crate::modules::settings::{Section, Tab};

    #[test]
    fn every_page_derives_its_height_from_the_shared_row_pitch() {
        let font_size = 16.0;
        let config = Config::default();
        assert_eq!(
            appearance::desired_height(font_size, false, false),
            style::page_height(appearance::rows(false, false), font_size)
        );
        assert_eq!(
            modules::desired_height(&config, font_size, Section::Left),
            style::page_height(modules::rows(&config, Section::Left), font_size)
        );
    }

    #[test]
    fn every_page_reserves_the_shared_label_column() {
        let font_size = 16.0;
        let config = Config::default();
        let column = style::label_width(font_size);

        assert!(appearance::desired_width(font_size) > column);
        assert!(modules::desired_width(&config, font_size, Section::Left) > column);
    }

    #[test]
    fn a_larger_text_size_makes_every_page_taller() {
        let config = Config::default();

        assert!(
            appearance::desired_height(20.0, false, false)
                > appearance::desired_height(16.0, false, false)
        );
        assert!(
            modules::desired_height(&config, 20.0, Section::Left)
                > modules::desired_height(&config, 16.0, Section::Left)
        );
    }

    #[test]
    fn the_window_reserves_a_row_for_its_header_and_for_its_tabs() {
        let config = Config::default();
        let settings = Settings::default();
        let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);

        assert!(
            settings.content_height(&config)
                >= 2.0 * style::row_height(font_size)
                    + appearance::desired_height(
                        font_size,
                        config.appearance.auto_scale,
                        config.updates.is_some()
                    )
        );
    }

    #[test]
    fn a_page_draws_into_less_room_than_the_window_asks_for() {
        let config = Config::default();
        let settings = Settings::default();

        assert!(settings.window_metrics(&config).page_width < settings.content_width(&config));
    }

    #[test]
    fn the_header_fits_the_title_it_draws() {
        let font_size = 16.0;
        let config = Config::default();
        let settings = Settings::default();

        assert!(settings.content_width(&config) >= metrics::text_width(TITLE, font_size));
    }

    #[test]
    fn every_tab_asks_for_a_positive_size() {
        let config = Config::default();

        for tab in Tab::ALL {
            let settings = Settings {
                tab,
                ..Settings::default()
            };

            assert!(settings.content_width(&config) > 0.0);
            assert!(settings.content_height(&config) > 0.0);
        }
    }
}
