//! One column of the unfolded bar: the modules of a section, come down.
//!
//! The section keeps its order and its groups; what changes is the form. A
//! module folded into a glance on the strip — a percentage, a time, a glyph —
//! stands on the canvas as the whole of what it knows: a heading, a rule
//! under it and a line per reading, which is the shape a desktop monitor has
//! always had.
//!
//! Four rooms. Here is the column itself, the stack a section becomes;
//! [`journey`] is where a block leaves from and how far it has to go,
//! [`runs`] breaks a section too deep for the screen into runs that stand
//! side by side,
//! [`order`] is what stands where in the stack, [`unit`] is the shape one
//! module takes on the canvas and [`panels`] is what each of them has to say.

mod journey;
mod order;
mod panels;
mod runs;
mod unit;

use hydebar_core::config::ModuleName;
use iced::{
    Element, Length, SurfaceId as Id,
    widget::{Column, container}
};

use super::{
    super::super::state::{App, Message},
    blocks::{Along, Ink, Side}
};

impl App {
    /// Stacks one section of the layout into a column of the canvas.
    ///
    /// The places are the same on every frame of the unfolding: the blocks
    /// are drawn away from them while they travel, by the journey each one
    /// carries, and nothing else moves.
    ///
    /// Every block carries its own journey: the one with least to go leaves
    /// first and the furthest last, so the strip empties as a run down the
    /// column rather than in one movement. They are kept apart by where they
    /// go as well as by when: each drops to its own level down the very line
    /// it stood on, which no other block is on, and only then closes in along
    /// its lane, by which time the levels are already a column apart.
    ///
    /// The gaps are the same size whatever the column holds. Pushing the far
    /// end of a short column down to the corner leaves a hole through the
    /// middle of the screen, and a column of everything the machine knows
    /// runs off the bottom edge instead: one honest gap, filled from the top,
    /// is what reads as a column either way.
    ///
    /// Returns nothing when no unit of the section has anything to draw, so
    /// an empty section leaves no gap on the canvas.
    #[expect(
        clippy::too_many_arguments,
        reason = "a column is drawn from what it holds, where it stands and how far it has come"
    )]
    pub(super) fn desk_column<'a>(
        &'a self,
        order: &[&'a ModuleName],
        id: Id,
        side: Side,
        ink: Ink,
        unfolding: f32,
        deepest: usize,
        room: f32
    ) -> Option<Element<'a, Message>> {
        let fan = self.fan_span();
        let runs = self.desk_runs(order, id, ink, room);

        if runs.is_empty() {
            return None;
        }

        let drawn: Vec<Element<'a, Message>> = runs
            .into_iter()
            .map(|run| {
                let blocks: Vec<Element<'a, Message>> = run
                    .into_iter()
                    .filter_map(|within| {
                        let unit = order[within];
                        let journey = Self::journey(within, deepest);
                        let (travel, bloom) = hydebar_core::animation::share(unfolding, journey);
                        let along = Along {
                            travel,
                            bloom,
                            glow: hydebar_core::animation::flare(unfolding, journey)
                        };
                        let block = self.desk_unit(unit, id, side, ink, along)?;

                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "a section holds a handful of units"
                        )]
                        let depth = within as f32 / (order.len().max(2) - 1) as f32;
                        let inwards = fan * (1.0 - depth);

                        let travelling = hydebar_core::components::flip::FlipAnchor::new(
                            self.flip_key(unit, id),
                            travel,
                            &self.flip,
                            block
                        );
                        let travelling = travelling
                            .departing_from(self.strip_row(id))
                            .descending_first();

                        Some(
                            container(travelling)
                                .width(Length::Fill)
                                .align_x(side.alignment_x())
                                .padding(side.lane(inwards))
                                .into()
                        )
                    })
                    .collect();

                Column::with_children(blocks)
                    .spacing(ink.size * 1.8)
                    .width(Length::Fill)
                    .align_x(side.alignment_x())
                    .into()
            })
            .collect();

        Some(
            iced::widget::Row::with_children(drawn)
                .spacing(ink.size * 2.0)
                .width(Length::Fill)
                .align_y(iced::Alignment::Start)
                .into()
        )
    }
}
