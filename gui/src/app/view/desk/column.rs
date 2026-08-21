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
    face, readings
};

impl App {
    /// Stacks one section of the layout into a column of the canvas.
    ///
    /// Returns nothing when no module of the section has anything to draw, so
    /// an empty section leaves no gap on the canvas.
    pub(super) fn desk_column<'a>(
        &'a self,
        section: &'a [ModuleDef],
        id: Id,
        side: Side,
        ink: Ink,
        travel: f32
    ) -> Option<Element<'a, Message>> {
        let blocks: Vec<Element<'a, Message>> = section
            .iter()
            .flat_map(|module_def| match module_def {
                ModuleDef::Single(module) => vec![module],
                ModuleDef::Group(group) => group.iter().collect()
            })
            .filter_map(|module| self.desk_block(module, id, side, ink))
            .collect();

        if blocks.is_empty() {
            return None;
        }

        Some(
            Column::with_children(blocks)
                .spacing(ink.size * 1.8 * travel)
                .width(Length::Fill)
                .align_x(side.alignment_x())
                .into()
        )
    }

    /// Draws one module in the form the canvas has room for.
    fn desk_block<'a>(
        &'a self,
        module: &'a ModuleName,
        id: Id,
        side: Side,
        ink: Ink
    ) -> Option<Element<'a, Message>> {
        if matches!(module, ModuleName::Clock) {
            return Some(self.desk_clock(ink, side));
        }

        let panels = self.desk_panels(module);

        if panels.is_empty() {
            return self.desk_reading(module, id);
        }

        Some(
            Column::with_children(panels.iter().map(|panel| blocks::panel(panel, side, ink)))
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
    fn desk_clock(&self, ink: Ink, side: Side) -> Element<'_, Message> {
        Column::new()
            .push(face::clock(
                self.clock.data(),
                &self.config.clock,
                ink.size,
                side.alignment_x()
            ))
            .push(self.calendar.menu_view(self.icons()).map(Message::Calendar))
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
