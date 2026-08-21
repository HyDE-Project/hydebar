//! One column of the unfolded bar: the modules of a section, come down.
//!
//! The section keeps its order and its groups; what changes is the form. A
//! module folded into a glance on the strip — a percentage, a time, a glyph —
//! stands on the canvas as the whole of what it knows: a heading, a rule
//! under it and a line per reading, which is the shape a desktop monitor has
//! always had.

use hydebar_core::config::{ModuleDef, ModuleName};
use iced::{
    Element, Length, SurfaceId as Id,
    widget::{Column, Space}
};

use super::{
    super::super::state::{App, Message},
    blocks::{self, Ink, Side},
    readings
};

/// How far the clock has to have opened before its month stands under it.
///
/// The face is one line and the month is six: letting them arrive together
/// would have the whole column jump the moment the block opens, so the hour
/// lands first and the month follows it.
const MONTH_OPENS_AT: f32 = 0.45;

impl App {
    /// Stacks one section of the layout into a column of the canvas.
    ///
    /// Returns nothing when no module of the section has anything to draw, so
    /// an empty section leaves no gap on the canvas.
    pub(super) fn desk_column<'a>(
        &'a self,
        order: &[&'a ModuleName],
        id: Id,
        side: Side,
        ink: Ink,
        travel: f32,
        bloom: f32
    ) -> Option<Element<'a, Message>> {
        let blocks: Vec<Element<'a, Message>> = order
            .iter()
            .filter_map(|module| {
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

        let reaches_the_corner = side.reaches_the_corner() && blocks.len() > 1;
        let mut column = Column::new()
            .width(Length::Fill)
            .align_x(side.alignment_x());

        for (index, block) in blocks.into_iter().enumerate() {
            if index > 0 {
                column = if reaches_the_corner {
                    column.push(Space::new().height(Length::Fill))
                } else {
                    column.push(Space::new().height(Length::Fixed(ink.size * 1.8 * travel)))
                };
            }

            column = column.push(block);
        }

        Some(if reaches_the_corner {
            column.height(Length::Fill).into()
        } else {
            column.into()
        })
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
