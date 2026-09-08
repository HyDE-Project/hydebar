//! The frame: the ring around the screen.
//!
//! The surface covers the whole screen but paints only the strips the user
//! switched on, in the strip's own ground — [`App::strip_ground`], the very
//! colour the bar's backdrop paints the strip in, `bar_opacity` on island
//! styles and `opacity` on the rest. One ground for the strip and the ring:
//! whatever the bar looks like, the ring is the same paint continued around
//! the screen, and it parts together with the strip's own wash. Nothing is
//! painted while the frame is disabled, the strip is gone, or the
//! compositor has not reported this screen's size yet.
//!
//! The geometry is the output's own measured logical size, never a screen
//! another output reported: a 4K panel and a 1080p one ring themselves each
//! in their own numbers.

use hydebar_core::{
    HEIGHT,
    frame::{geometry::frame_pieces, program::FrameCanvas}
};
use iced::{Element, Length, SurfaceId as Id, widget::canvas};

use super::super::state::{App, Message};

impl App {
    /// Draws the frame of one output: the enabled edge strips, or nothing.
    pub(super) fn frame_surface(&self, id: Id) -> Element<'_, Message> {
        let frame = &self.config.appearance.frame;
        let empty = iced::widget::Row::new().into();

        if !frame.enabled {
            return empty;
        }

        let Some(name) = self.outputs.screen_of(id).flatten() else {
            return empty;
        };

        let Some(&(screen_w, screen_h)) = self.screen_sizes.get(name) else {
            return empty;
        };

        let wash = self.strip_wash(Some(name));

        if wash <= 0.0 {
            return empty;
        }

        let appearance = self.appearance();
        let theme = &self.theme_cache;
        let color = self.strip_ground(theme);

        #[expect(
            clippy::cast_possible_truncation,
            reason = "the bar height constant is exactly representable in f32"
        )]
        let bar_height = appearance.height_px().unwrap_or(HEIGHT as f32);

        let shape = frame_pieces(screen_w, screen_h, frame, self.config.position, bar_height);

        log::debug!(
            "frame: drawing the ring of {name} over {screen_w}x{screen_h}, wash {wash}, shape {shape:?}"
        );

        canvas(FrameCanvas {
            shape,
            color
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
