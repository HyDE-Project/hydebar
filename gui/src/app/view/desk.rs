//! The desk surface: the canvas the bar unfolds into on a bare screen.
//!
//! Three columns over the wallpaper, in the shape the old desktop monitors
//! kept: the machine and its link down the left edge, the hour and the sky in
//! the middle, the load and the mounts down the right. It is drawn only while
//! the screen the surface stands on holds no window at all, so it never
//! competes with anything for attention — there is nothing else there.
//!
//! One folder, three rooms: [`readings`] settles what each panel says,
//! [`blocks`] draws one panel and [`hour`] draws the middle.

mod blocks;
mod hour;
mod readings;

use blocks::{Ink, Side};
use hydebar_proto::config::DeskPanel;
use iced::{
    Alignment, Element, Length, SurfaceId as Id,
    widget::{Column, Row, container}
};

use super::super::state::{App, Message};

impl App {
    /// Draws the desk of one output, or nothing while a window holds it.
    ///
    /// The canvas unfolds rather than appears: it fades in over the wallpaper
    /// and rises the last stretch into place, and folds back the same way the
    /// moment a window maps. The travel is the screen's own spring, so a
    /// second monitor still holding a window is untouched by it.
    pub(super) fn desk_surface(&self, id: Id) -> Element<'_, Message> {
        let screen = self.outputs.screen_of(id).flatten();
        let presence = self.desk_presence(screen);

        if !self.config.desk.enabled || presence <= 0.004 {
            return Row::new().into();
        }

        let ink = Ink {
            value: self.theme_cache.palette().text.scale_alpha(presence),
            size:  self.appearance().font_size_px()
        };
        let desk = &self.config.desk;
        let margin = ink.size * 2.0;
        let rise = margin.mul_add(1.0 - presence, margin);

        let columns = [
            (&desk.left, Side::Leading),
            (&desk.center, Side::Leading),
            (&desk.right, Side::Trailing)
        ]
        .into_iter()
        .map(|(panels, side)| self.desk_column(panels, side, ink));

        container(
            Row::with_children(columns)
                .spacing(ink.size * 4.0)
                .align_y(Alignment::Start)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding {
            top:    rise,
            right:  margin,
            bottom: margin,
            left:   margin
        })
        .into()
    }

    /// Stacks one column of the canvas, empty panels left out.
    fn desk_column(&self, panels: &[DeskPanel], side: Side, ink: Ink) -> Element<'_, Message> {
        Column::with_children(
            panels
                .iter()
                .filter_map(|panel| self.desk_panel(*panel, side, ink))
        )
        .spacing(ink.size * 1.8)
        .width(Length::FillPortion(1))
        .align_x(side.alignment_x())
        .into()
    }

    /// Draws one panel, or nothing when the machine reports nothing for it.
    fn desk_panel(&self, panel: DeskPanel, side: Side, ink: Ink) -> Option<Element<'_, Message>> {
        let data = self.system_info.data();

        let reading = match panel {
            DeskPanel::System => readings::system(data),
            DeskPanel::Network => readings::network(data),
            DeskPanel::Processor => readings::processor(data),
            DeskPanel::Graphics => readings::graphics(data),
            DeskPanel::Memory => readings::memory(data),
            DeskPanel::Storage => readings::storage(data),
            DeskPanel::Clock => {
                return Some(hour::clock(self.clock.data(), &self.config.clock, ink));
            }
            DeskPanel::Weather => {
                let sky = self.weather.data();

                return sky.has_reading().then(|| hour::weather(sky, ink));
            }
        };

        reading.map(|reading| blocks::panel(&reading, side, ink))
    }
}
