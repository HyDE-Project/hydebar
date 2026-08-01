//! The menu window: its render, and the three lengths the compositor is
//! told before anything inside it has been laid out.

use hydebar_proto::config::Config;
use iced::Element;

use super::{Message, Themes, view};
use crate::components::page;

impl Themes {
    /// Renders the menu the module opens.
    ///
    /// `opacity` is the menu opacity the surface is animating through, so the
    /// chips fade in with the box that holds them.
    #[must_use]
    pub fn menu_view<'a>(
        &self,
        config: &Config,
        opacity: f32,
        page_width: f32
    ) -> Element<'a, Message> {
        let font_size = config.appearance.font_size_px();

        view::view(
            &self.hyde,
            &self.swatches,
            &self.screenshots,
            self.switching(),
            &self.catalogue,
            &self.offered,
            &self.catalogue_index,
            self.author.as_deref(),
            self.installing.as_deref(),
            self.updating.as_ref(),
            self.list_layout,
            self.spinner,
            opacity,
            font_size,
            page_width
        )
    }

    /// The three window lengths, with the content walked exactly once.
    #[must_use]
    pub fn window_metrics(&self, config: &Config) -> crate::menu::MenuMetrics {
        let font_size = config.appearance.font_size_px();
        let width = self.content_width(config);
        let page_width = page::metrics::ROW_SLACK_EM.mul_add(-font_size, width);

        crate::menu::MenuMetrics {
            width,
            page_width,
            height: view::desired_height(
                &self.hyde,
                &self.offered,
                self.list_layout,
                font_size,
                page_width
            )
        }
    }

    /// Width the longest row of the menu needs.
    ///
    /// Measured rather than guessed for the same reason the settings window
    /// measures itself: the compositor is told how large the surface is before
    /// anything inside it has been laid out.
    #[must_use]
    pub fn content_width(&self, config: &Config) -> f32 {
        let font_size = config.appearance.font_size_px();

        page::metrics::ROW_SLACK_EM.mul_add(
            font_size,
            view::desired_width(&self.hyde, self.switching(), font_size)
        )
    }

    /// Height the menu needs.
    #[must_use]
    pub fn content_height(&self, config: &Config) -> f32 {
        self.window_metrics(config).height
    }
}
