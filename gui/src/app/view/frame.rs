//! The frame: the ring around the screen.
//!
//! The surface covers the whole screen but paints only the strips the user
//! switched on, in the strip's own ground — [`App::strip_ground`], the very
//! colour the bar's backdrop paints the strip in, `bar_opacity` on island
//! styles and `opacity` on the rest. One ground for the strip and the ring:
//! whatever the bar looks like, the ring is the same paint continued around
//! the screen, and it parts together with the strip's own wash. Nothing is
//! painted while the frame is disabled, the strip is gone, or the
//! compositor has not reported the screen size yet.

use hydebar_core::{
    HEIGHT,
    frame::{geometry::frame_pieces, program::FrameCanvas}
};
use iced::{Element, Length, widget::canvas};

use super::super::state::{App, Message};

impl App {
    /// Draws the frame of one output: the enabled edge strips, or nothing.
    pub(super) fn frame_surface(&self) -> Element<'_, Message> {
        let frame = &self.config.appearance.frame;

        log::debug!(
            "frame surface view: enabled={}, screen={:?}x{:?}",
            frame.enabled,
            self.screen_width,
            self.screen_height
        );

        let (Some(screen_w), Some(screen_h)) = (self.screen_width, self.screen_height) else {
            return iced::widget::Row::new().into();
        };

        if !frame.enabled {
            return iced::widget::Row::new().into();
        }

        let appearance = self.appearance();
        let screen = self.outputs.screens().next().flatten();
        let wash = self.strip_wash(screen);

        if wash <= 0.0 {
            return iced::widget::Row::new().into();
        }

        let theme = &self.theme_cache;
        let color = self.strip_ground(theme);

        #[expect(
            clippy::cast_possible_truncation,
            reason = "the bar height constant is exactly representable in f32"
        )]
        let bar_height = appearance.height_px().unwrap_or(HEIGHT as f32);

        let pieces = frame_pieces(screen_w, screen_h, frame, self.config.position, bar_height);

        log::debug!(
            "frame: enabled, drawing {} piece(s) over {}x{}, wash {wash}",
            pieces.len(),
            screen_w,
            screen_h
        );

        canvas(FrameCanvas { pieces, color })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
