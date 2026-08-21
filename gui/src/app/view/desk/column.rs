//! One column of the unfolded bar: the modules of a section, come down.
//!
//! The section keeps its order and its groups; what changes is the form. A
//! module folded into a glance on the strip — a percentage, a time, a glyph —
//! stands on the canvas as the whole of what it knows: a heading, a rule
//! under it and a line per reading, which is the shape a desktop monitor has
//! always had.

use hydebar_core::config::{ModuleDef, ModuleName};
use iced::{Element, Length, SurfaceId as Id, widget::Column};

use super::{
    super::super::state::{App, Message},
    blocks::{self, Ink, Side},
    readings
};

/// The share of one block's journey spent crossing the screen.
///
/// The rest is spent writing itself out. Stated as one journey rather than
/// two because a block that arrives and then waits for a second animation to
/// be started reads as a stutter: the crossing runs into the opening with
/// nothing in between.
const CROSSING: f32 = 0.55;

/// The stagger that keeps two blocks from ever being in flight together.
///
/// Blocks leave the strip from one row and arrive in a column, so their paths
/// cross: whatever is still travelling would be drawn over whatever is
/// already there. One at a time is the rule that removes the question — the
/// block nearest the middle of the strip goes first and the next one starts
/// when it has arrived.
///
/// The share is what makes each window disjoint: with `blocks` of them the
/// windows are `1 - spread` wide and start `spread / (blocks - 1)` apart, so
/// a spread of `(blocks - 1) / blocks` leaves them just touching.
#[expect(
    clippy::cast_precision_loss,
    reason = "a column holds a handful of blocks, far below any precision limit"
)]
fn stagger(blocks: usize) -> f32 {
    if blocks < 2 {
        return 0.0;
    }

    let blocks = blocks as f32;

    (blocks - 1.0) / blocks
}

/// How far one block has crossed and how far it has opened.
///
/// `place` is where the block stands in its column: zero leads the front, one
/// trails it. `blocks` is how many share the column, which is what sets the
/// stagger that keeps their flights apart.
pub(super) fn journey(unfolding: f32, place: f32, blocks: usize) -> (f32, f32) {
    let own = hydebar_core::animation::sweep(unfolding, place, stagger(blocks));

    if own <= CROSSING {
        (own / CROSSING, 0.0)
    } else {
        (1.0, (own - CROSSING) / (1.0 - CROSSING))
    }
}

/// How far the clock has to have opened before its month stands under it.
///
/// The face is one line and the month is six: letting them arrive together
/// would have the whole column jump the moment the block opens, so the hour
/// lands first and the month follows it.
const MONTH_OPENS_AT: f32 = 0.45;

impl App {
    /// Stacks one section of the layout into a column of the canvas.
    ///
    /// The places are the same on every frame of the unfolding: the blocks
    /// are drawn away from them while they travel, by the journey each one
    /// carries, and nothing else moves. A canvas whose own padding and gaps
    /// also grew with the travel would be moving the places the blocks are
    /// aiming at, which reads as a stagger rather than as a journey.
    ///
    /// The gaps are the same size whatever the column holds. Pushing the far
    /// end of a short column down to the corner leaves a hole through the
    /// middle of the screen, and a column of everything the machine knows
    /// runs off the bottom edge instead: one honest gap, filled from the top,
    /// is what reads as a column either way.
    ///
    /// Returns nothing when no module of the section has anything to draw, so
    /// an empty section leaves no gap on the canvas.
    pub(super) fn desk_column<'a>(
        &'a self,
        order: &[&'a ModuleName],
        id: Id,
        side: Side,
        ink: Ink,
        unfolding: f32
    ) -> Option<Element<'a, Message>> {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a section holds a handful of modules, far below any precision limit"
        )]
        let last = (order.len().saturating_sub(1)) as f32;

        let blocks: Vec<Element<'a, Message>> = order
            .iter()
            .enumerate()
            .filter_map(|(index, module)| {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a section holds a handful of modules"
                )]
                let place = if last > 0.0 { index as f32 / last } else { 0.0 };
                let (travel, bloom) = journey(unfolding, place, order.len());

                let block = self.desk_block(module, id, side, ink, bloom)?;

                Some(
                    hydebar_core::components::flip::FlipAnchor::new(
                        self.flip_key(module, id),
                        travel,
                        &self.flip,
                        block
                    )
                    .departing_from(self.strip_row())
                    .into()
                )
            })
            .collect();

        if blocks.is_empty() {
            return None;
        }

        Some(
            Column::with_children(blocks)
                .spacing(ink.size * 1.8)
                .width(Length::Fill)
                .align_x(side.alignment_x())
                .into()
        )
    }

    /// The modules of one section, in the order the canvas stands them in.
    ///
    /// The rule is the distance from the middle of the strip: a module that
    /// stood near the centre stands high on the canvas, and one that stood at
    /// an edge reaches for the corner below it. The centre section already
    /// reads outwards from the middle; the left one reads towards it, so it
    /// is turned around.
    pub(crate) fn desk_order(
        section: &[ModuleDef],
        reads_towards_the_centre: bool
    ) -> Vec<&ModuleName> {
        let mut order: Vec<&ModuleName> = section
            .iter()
            .flat_map(|module_def| match module_def {
                ModuleDef::Single(module) => vec![module],
                ModuleDef::Group(group) => group.iter().collect()
            })
            .collect();

        if reads_towards_the_centre {
            order.reverse();
        }

        order
    }

    /// The height the strip's own islands stand at.
    ///
    /// Where every block departs from: the canvas covers the whole screen,
    /// strip band included, so the row the modules leave is the top of the
    /// canvas rather than somewhere above it.
    fn strip_row(&self) -> f32 {
        self.appearance().bar_padding()[0]
    }

    /// Draws one module in the form the canvas has room for.
    fn desk_block<'a>(
        &'a self,
        module: &'a ModuleName,
        id: Id,
        side: Side,
        ink: Ink,
        bloom: f32
    ) -> Option<Element<'a, Message>> {
        let island = self.desk_island(module, id)?;

        if bloom <= 0.0 {
            return Some(island);
        }

        let opened: Vec<Element<'a, Message>> = if matches!(module, ModuleName::Clock) {
            self.desk_month(bloom).into_iter().collect()
        } else {
            self.desk_panels(module)
                .iter()
                .map(|panel| blocks::panel(panel, side, ink, bloom))
                .collect()
        };

        if opened.is_empty() {
            return Some(island);
        }

        Some(
            Column::with_children(std::iter::once(island).chain(opened))
                .spacing(ink.size * 0.9)
                .width(Length::Fill)
                .align_x(side.alignment_x())
                .into()
        )
    }

    /// What a module has to say once it has the room to say all of it.
    ///
    /// One arm per module, and every arm answers from the state that module
    /// already keeps: the desk samples nothing of its own. A module that has
    /// not answered yet, or has nothing longer to say than its strip entry,
    /// yields no panel and stands on the canvas as it stands on the strip.
    fn desk_panels(&self, module: &ModuleName) -> Vec<readings::Panel> {
        let machine = self.system_info.data();

        match module {
            ModuleName::SystemInfo => [
                readings::system(machine),
                readings::processor(machine),
                readings::graphics(machine),
                readings::memory(machine),
                readings::storage(machine),
                readings::network(machine)
            ]
            .into_iter()
            .flatten()
            .collect(),
            ModuleName::Cpu => readings::processor(machine).into_iter().collect(),
            ModuleName::CpuTemp => readings::cpu_temperature(machine).into_iter().collect(),
            ModuleName::GpuTemp => readings::graphics(machine).into_iter().collect(),
            ModuleName::Memory => readings::memory(machine).into_iter().collect(),
            ModuleName::Battery => readings::battery(self).into_iter().collect(),
            ModuleName::Updates => readings::updates(self).into_iter().collect(),
            ModuleName::Notifications => readings::notifications(self).into_iter().collect(),
            ModuleName::Privacy => readings::privacy(self).into_iter().collect(),
            ModuleName::KeyboardLayout => readings::keyboard(self).into_iter().collect(),
            ModuleName::Weather => readings::weather(self).into_iter().collect(),
            ModuleName::Tray => readings::tray(self).into_iter().collect(),
            ModuleName::Themes => readings::theme(self).into_iter().collect(),
            ModuleName::ControlCenter | ModuleName::IdleInhibitor => {
                readings::session_idle(self).into_iter().collect()
            }
            _ => Vec::new()
        }
    }

    /// The month the clock opens into, once it has opened far enough.
    ///
    /// The very grid its press opens on the strip — the same widget, drawn
    /// straight onto the wallpaper instead of into a popup. It waits for the
    /// opening to be under way because it is six rows tall against the one
    /// row of the island above it.
    fn desk_month(&self, bloom: f32) -> Option<Element<'_, Message>> {
        (bloom >= MONTH_OPENS_AT)
            .then(|| self.calendar.menu_view(self.icons()).map(Message::Calendar))
    }

    /// The island the module arrived on the canvas as.
    ///
    /// The very thing that travelled: the strip's own view of the module, in
    /// the pill the strip drew around it. It is not swapped for a heading
    /// once the block opens — the block grows underneath it — because a
    /// module that vanished at the end of its own journey would undo the
    /// journey.
    fn desk_island<'a>(&'a self, module: &'a ModuleName, id: Id) -> Option<Element<'a, Message>> {
        let opacity = self.appearance().opacity;
        let (content, action) = self.get_module_view(module, id, opacity)?;
        let actions = self.module_actions(module, action);

        Some(self.module_element(content, actions, module, id, false))
    }
}
