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
    face, readings
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
            .filter_map(|module| self.desk_block(module, id, side, ink, bloom))
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
    pub(super) fn desk_order(
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

    /// Draws one module in the form the canvas has room for.
    fn desk_block<'a>(
        &'a self,
        module: &'a ModuleName,
        id: Id,
        side: Side,
        ink: Ink,
        bloom: f32
    ) -> Option<Element<'a, Message>> {
        if bloom <= 0.0 {
            return self.desk_reading(module, id);
        }

        if matches!(module, ModuleName::Clock) {
            return Some(self.desk_clock(ink, side, bloom));
        }

        let panels = self.desk_panels(module);

        if panels.is_empty() {
            return self.desk_reading(module, id);
        }

        Some(
            Column::with_children(
                panels
                    .iter()
                    .map(|panel| blocks::panel(panel, side, ink, bloom))
            )
            .spacing(ink.size * 1.4)
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

    /// The clock: its hour at the size of the canvas, its month under it.
    ///
    /// The month is the very grid its press opens on the strip — the same
    /// widget, drawn straight onto the wallpaper instead of into a popup.
    fn desk_clock(&self, ink: Ink, side: Side, bloom: f32) -> Element<'_, Message> {
        let face = face::clock(
            self.clock.data(),
            &self.config.clock,
            ink.size,
            side.alignment_x()
        );

        let clock = Column::new().push(face);

        let clock = if bloom >= MONTH_OPENS_AT {
            clock.push(self.calendar.menu_view(self.icons()).map(Message::Calendar))
        } else {
            clock
        };

        clock
            .spacing(ink.size)
            .width(Length::Fill)
            .align_x(side.alignment_x())
            .into()
    }

    /// The strip's own view of a module, presses and all.
    ///
    /// What a module with nothing longer to say stands as: a launcher, a
    /// picker, the workspace row.
    fn desk_reading<'a>(&'a self, module: &'a ModuleName, id: Id) -> Option<Element<'a, Message>> {
        let opacity = self.appearance().opacity;
        let (content, action) = self.get_module_view(module, id, opacity)?;
        let actions = self.module_actions(module, action);

        Some(self.module_element(content, actions, module, id, true))
    }
}
