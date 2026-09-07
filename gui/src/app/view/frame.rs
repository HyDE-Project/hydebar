//! The frame: the ring around the screen, painted on the desk surface.
//!
//! The surface covers the whole screen but paints only the strips the user
//! switched on, in the exact paint the island buttons rest at — the bar
//! palette's background at the island opacity, read from the same theme
//! cache the buttons draw from. One source of truth, not two matching
//! numbers: a theme morph repaints the ring with the bar, frame by frame.
//! With the frame disabled, or before the compositor reports the screen
//! size, nothing is painted.

use hydebar_core::{
    HEIGHT,
    config::Appearance,
    frame::{geometry::frame_pieces, program::FrameCanvas}
};
use iced::{Element, Length, widget::canvas};

use super::super::state::{App, Message};

impl App {
    /// Draws the frame of one output: the enabled edge strips, or nothing.
    pub(super) fn frame_surface(&self) -> Element<'_, Message> {
        let frame = &self.config.appearance.frame;

        let (Some(screen_w), Some(screen_h)) = (self.screen_width, self.screen_height) else {
            return iced::widget::Row::new().into();
        };

        if !frame.enabled {
            return iced::widget::Row::new().into();
        }

        let appearance: &Appearance = self.appearance();
        let color = self
            .theme_cache
            .palette()
            .background
            .scale_alpha(appearance.opacity);

        #[expect(
            clippy::cast_possible_truncation,
            reason = "the bar height constant is exactly representable in f32"
        )]
        let bar_height = appearance.height_px().unwrap_or(HEIGHT as f32);

        let pieces = frame_pieces(screen_w, screen_h, frame, self.config.position, bar_height);

        canvas(FrameCanvas {
            pieces,
            color
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
