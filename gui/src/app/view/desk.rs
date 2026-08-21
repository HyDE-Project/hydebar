//! The desk surface: the shape the bar takes when the screen is bare.
//!
//! Not a second bar and not a second set of readouts — the same modules, in
//! the same layout, come down off the strip and stand over the wallpaper at a
//! size a room away can read. The three sections of the layout become three
//! columns, each keeping its own order and its own groups, and each module
//! keeps the presses it answers to on the strip.
//!
//! One folder, one room so far: [`column`] stacks a section into a column.

mod blocks;
mod column;
mod face;
mod readings;

use hydebar_core::outputs::HasOutput;
use iced::{
    Alignment, Element, Length, Padding, SurfaceId as Id,
    widget::{Row, container}
};

use super::super::state::{App, Message};

impl App {
    /// Draws the unfolded bar of one output, or nothing while a window holds
    /// the screen.
    ///
    /// The canvas unfolds rather than appears: it fades in over the wallpaper
    /// and rises the last stretch into place while the strip fades out under
    /// it, and folds back the same way the moment a window maps. The travel is
    /// the screen's own spring, so a second monitor still holding a window is
    /// untouched by it.
    pub(super) fn desk_surface(&self, id: Id) -> Element<'_, Message> {
        let screen = self.outputs.screen_of(id).flatten();
        let presence = self.desk_presence(screen);

        if !self.config.desk.enabled || presence <= 0.004 {
            return Row::new().into();
        }

        let ink = blocks::Ink {
            value: self.theme_cache.palette().text,
            size:  self.appearance().font_size_px()
        };
        let margin = ink.size * 2.0;
        let modules = &self.config.modules;

        let columns = [
            (&modules.left, blocks::Side::Leading),
            (&modules.center, blocks::Side::Leading),
            (&modules.right, blocks::Side::Trailing)
        ]
        .into_iter()
        .filter_map(|(section, side)| self.desk_column(section, id, side, ink));

        let canvas = container(
            Row::with_children(columns)
                .spacing(margin)
                .align_y(Alignment::Start)
                .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top:    margin.mul_add(1.0 - presence, margin),
            right:  margin,
            bottom: margin,
            left:   margin
        });

        self.faded_menu(canvas.into(), presence)
    }

    /// How much larger than the strip the canvas of `id` is drawn.
    ///
    /// One for every surface but the desk: the modules are drawn from the
    /// same views the strip uses, and the whole surface is magnified instead,
    /// which is what lets one layout serve a thirty pixel strip and a whole
    /// screen without a second set of sizes.
    pub(crate) fn desk_magnification(&self, id: Id) -> f64 {
        if !self.config.desk.enabled || !matches!(self.outputs.has(id), Some(HasOutput::Desk)) {
            return 1.0;
        }

        f64::from(self.config.desk.magnification())
    }
}
