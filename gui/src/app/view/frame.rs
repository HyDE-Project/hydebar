//! The frame surface: the ring around the screen.
//!
//! The surface covers the whole screen but paints only the strips the user
//! switched on, in the bar's own background colour at the frame's own
//! opacity — so the frame reads as one surface with the bar rather than as
//! a second bar. With the frame disabled, or before the compositor reports
//! the screen size, the surface stays an empty drawing, exactly like a
//! tooltip surface with nothing to show.

use hydebar_core::{
    HEIGHT,
    config::Appearance,
    frame::{geometry::frame_pieces, program::FrameCanvas},
    style::painted
};
use hydebar_proto::theme_source::Rgba;
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
        let base = appearance.background_color.get_base();
        let color = painted(Rgba {
            a: frame.opacity,
            ..base
        });

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
