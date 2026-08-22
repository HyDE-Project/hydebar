//! The shape one module takes once it is down on the canvas.

use hydebar_core::config::ModuleName;
use iced::{
    Element, Length, SurfaceId as Id,
    widget::{Column, container}
};

use super::super::{
    super::super::state::{App, Message},
    blocks::{self, Ink, Side}
};

/// How wide a block is written, in body letters.
///
/// A reading is a label and a figure, and the eye pairs them by how close
/// they are. Given the whole third of a screen a column has, the label ends
/// up an arm's length from its figure and the pair stops being a pair — which
/// is why every table ever printed has a measure. This is that measure: wide
/// enough for the longest value the bar shows, narrow enough that the two
/// halves of a line still read as one line.
const MEASURE: f32 = 30.0;

impl App {
    /// Draws one unit in the form the canvas has room for.
    ///
    /// The opened block is built on every frame of the unfolding, empty of
    /// writing or full of it: it is what takes the unit's room in the column,
    /// and a unit that stood as a bare island until it began to open would
    /// take its room only then, moving everything below it down the screen
    /// mid-flight.
    pub(super) fn desk_unit<'a>(
        &'a self,
        unit: &'a ModuleName,
        id: Id,
        side: Side,
        ink: Ink,
        bloom: f32
    ) -> Option<Element<'a, Message>> {
        let island = self.desk_island(unit, id)?;
        let opened: Vec<Element<'a, Message>> = self.desk_opened(unit, side, ink, bloom);

        if opened.is_empty() {
            return Some(island);
        }

        Some(
            container(
                Column::with_children(std::iter::once(island).chain(opened))
                    .spacing(ink.size * 0.9)
                    .width(Length::Fill)
                    .align_x(side.alignment_x())
            )
            .max_width(ink.size * MEASURE)
            .into()
        )
    }

    /// Whether the unit draws anything at all on this screen.
    ///
    /// A unit whose module has nothing to draw takes no place in the column,
    /// so it takes no room either.
    pub(in crate::app::view::desk) fn desk_island_exists(
        &self,
        unit: &ModuleName,
        id: Id
    ) -> bool {
        self.desk_island(unit, id).is_some()
    }

    /// The room one unit takes when it is open, at the given ink.
    ///
    /// Worked out from the same figures that reserve it, so what this says a
    /// column needs is what the column takes: the island it arrived as, the
    /// gap under it, and the room of every block it opens into.
    pub(in crate::app::view::desk) fn desk_unit_room(&self, unit: &ModuleName, ink: Ink) -> f32 {
        let island = self.appearance().height.unwrap_or(38.0);
        let opened = self.desk_room(unit, ink);

        if opened <= 0.0 {
            return island;
        }

        ink.size.mul_add(0.9, island) + opened
    }

    /// The room the blocks of one unit take, without the island above them.
    fn desk_room(&self, unit: &ModuleName, ink: Ink) -> f32 {
        if matches!(unit, ModuleName::Clock) {
            let seat = super::super::readings::seat(self).map_or(0.0, |panel| {
                ink.size.mul_add(0.9, blocks::room_of(&panel, ink))
            });

            return ink.size.mul_add(blocks::MONTH_ROWS, seat);
        }

        let panels = self.desk_panels(unit);

        if panels.is_empty() {
            return blocks::blank_room(ink);
        }

        #[expect(
            clippy::cast_precision_loss,
            reason = "a unit opens into a handful of blocks"
        )]
        let gaps = (panels.len() - 1) as f32 * (ink.size * 0.9);

        panels
            .iter()
            .map(|panel| blocks::room_of(panel, ink))
            .sum::<f32>()
            + gaps
    }

    /// What one module of an opened unit writes out.
    fn desk_opened<'a>(
        &'a self,
        module: &'a ModuleName,
        side: Side,
        ink: Ink,
        bloom: f32
    ) -> Vec<Element<'a, Message>> {
        if matches!(module, ModuleName::Clock) {
            return std::iter::once(blocks::month(self.desk_month(), ink, bloom))
                .chain(
                    super::super::readings::seat(self)
                        .into_iter()
                        .map(|panel| blocks::panel(&panel, side, ink, bloom))
                )
                .collect();
        }

        let panels = self.desk_panels(module);

        if panels.is_empty() {
            return vec![blocks::awaited(module.label(), side, ink, bloom)];
        }

        panels
            .iter()
            .map(|panel| blocks::panel(panel, side, ink, bloom))
            .collect()
    }

    /// The island the unit arrived on the canvas as.
    ///
    /// The very thing that travelled, and it travels as the strip drew it: a
    /// module on its own carries its own pill, and a group carries the one
    /// pill its modules shared. It is not swapped for a heading once the
    /// block opens — the block grows underneath it — because a module that
    /// vanished at the end of its own journey would undo the journey.
    ///
    /// Its members are drawn as the strip draws a grouped module, which is
    /// the one that owns the height its own content needs. A module drawn as
    /// its own island fills the row it stands in, and a row of the canvas is
    /// as tall as the column: the island stretched down the screen and left
    /// its own readings behind.
    fn desk_island<'a>(&'a self, unit: &'a ModuleName, id: Id) -> Option<Element<'a, Message>> {
        let opacity = self.appearance().opacity;
        let (content, action) = self.get_module_view(unit, id, opacity)?;
        let actions = self.module_actions(unit, action);

        Some(self.desk_pill(vec![self.module_element(content, actions, unit, id, true)]))
    }

    /// The one pill a group of modules shares, as the strip paints it.
    fn desk_pill<'a>(&'a self, members: Vec<Element<'a, Message>>) -> Element<'a, Message> {
        use hydebar_proto::config::AppearanceStyle;

        let appearance = self.appearance();
        let row = iced::widget::Row::with_children(members)
            .spacing(appearance.island_gap())
            .align_y(iced::Alignment::Center);

        if appearance.style != AppearanceStyle::Islands {
            return row.into();
        }

        let opacity = appearance.opacity;
        let finish = hydebar_core::style::IslandFinish::of(appearance);
        let radius = appearance.pill_radius();

        container(row)
            .padding(appearance.island_padding())
            .style(move |theme: &iced::Theme| iced::widget::container::Style {
                background: Some(theme.palette().background.scale_alpha(opacity).into()),
                border: finish.border(radius),
                shadow: finish.shadow(),
                ..iced::widget::container::Style::default()
            })
            .into()
    }

    /// The month the clock opens into.
    ///
    /// The very grid its press opens on the strip — the same widget, drawn
    /// straight onto the wallpaper instead of into a popup. Its room is taken
    /// from the first frame of the unfolding and it is written into that room
    /// as the clock lands, the same as every other block: standing there
    /// whole from the first frame was the one thing on the canvas that did
    /// not open.
    fn desk_month(&self) -> Element<'_, Message> {
        self.calendar.menu_view(self.icons()).map(Message::Calendar)
    }
}
