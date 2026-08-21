//! One column of the unfolded bar: the modules of a section, come down.
//!
//! The section keeps everything the strip gave it — its order, its groups,
//! its presses — and only the direction changes: what stood side by side in a
//! row stands one under the other, because that is what a screen has room
//! for and a strip does not.

use hydebar_core::config::{ModuleDef, ModuleName};
use iced::{
    Alignment, Element, Length, SurfaceId as Id,
    widget::{Column, Row}
};

use super::super::super::state::{App, Message};

impl App {
    /// Stacks one section of the layout into a column of the canvas.
    ///
    /// Returns nothing when no module of the section has anything to draw,
    /// so an empty section leaves no gap on the canvas.
    pub(super) fn desk_column<'a>(
        &'a self,
        section: &'a [ModuleDef],
        id: Id,
        align: Alignment
    ) -> Option<Element<'a, Message>> {
        let opacity = self.appearance().opacity;

        let rows: Vec<Element<'a, Message>> = section
            .iter()
            .filter_map(|module_def| self.desk_row(module_def, id, opacity, align))
            .collect();

        if rows.is_empty() {
            return None;
        }

        Some(
            Column::with_children(rows)
                .spacing(self.appearance().font_size_px() * 1.4)
                .width(Length::Fill)
                .align_x(align)
                .into()
        )
    }

    /// Draws one entry of the layout: a module on its own, or a whole group.
    ///
    /// A group stays a row of its own on the canvas. The layout put those
    /// modules side by side on purpose — a clock beside its battery — and
    /// unfolding must not be what pulls them apart.
    fn desk_row<'a>(
        &'a self,
        module_def: &'a ModuleDef,
        id: Id,
        opacity: f32,
        align: Alignment
    ) -> Option<Element<'a, Message>> {
        let names: Vec<&'a ModuleName> = match module_def {
            ModuleDef::Single(module) => vec![module],
            ModuleDef::Group(group) => group.iter().collect()
        };

        let drawn: Vec<Element<'a, Message>> = names
            .into_iter()
            .filter_map(|name| self.desk_module(name, id, opacity))
            .collect();

        if drawn.is_empty() {
            return None;
        }

        Some(
            Row::with_children(drawn)
                .spacing(self.appearance().font_size_px() * 0.8)
                .align_y(Alignment::Center)
                .width(Length::Shrink)
                .into()
        )
        .map(|row: Element<'a, Message>| {
            iced::widget::container(row)
                .width(Length::Fill)
                .align_x(align)
                .into()
        })
    }

    /// Draws one module of the canvas, presses and all.
    ///
    /// The same view the strip draws, wrapped in the same button: the desk is
    /// the bar in another shape, so a module that opens a menu on the strip
    /// opens the same menu here.
    fn desk_module<'a>(
        &'a self,
        module_name: &'a ModuleName,
        id: Id,
        opacity: f32
    ) -> Option<Element<'a, Message>> {
        let (content, action) = self.get_module_view(module_name, id, opacity)?;
        let actions = self.module_actions(module_name, action);

        Some(self.module_element(content, actions, module_name, id, true))
    }
}
