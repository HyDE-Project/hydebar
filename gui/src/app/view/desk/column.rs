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
    widget::{Column, container}
};

use super::{
    super::super::state::{App, Message},
    blocks::{self, Ink, Side},
    readings
};

/// The share of the screen the fan of one section reaches across.
///
/// Wide enough that the lanes are plainly apart and narrow enough that the
/// two edge sections never meet in the middle, where the centre one stands.
const FAN: f32 = 0.16;

impl App {
    /// Stacks one section of the layout into a column of the canvas.
    ///
    /// The places are the same on every frame of the unfolding: the blocks
    /// are drawn away from them while they travel, by the journey each one
    /// carries, and nothing else moves.
    ///
    /// Every block of the column carries the same journey, so the whole bar
    /// leaves at one instant and none of them waits on another. They are kept
    /// apart by where they go rather than by when they go: each drops to its
    /// own level down the very line it stood on, which no other block is on,
    /// and only then closes in along its lane, by which time the levels are
    /// already a column apart.
    ///
    /// The gaps are the same size whatever the column holds. Pushing the far
    /// end of a short column down to the corner leaves a hole through the
    /// middle of the screen, and a column of everything the machine knows
    /// runs off the bottom edge instead: one honest gap, filled from the top,
    /// is what reads as a column either way.
    ///
    /// Returns nothing when no unit of the section has anything to draw, so
    /// an empty section leaves no gap on the canvas.
    pub(super) fn desk_column<'a>(
        &'a self,
        order: &[&'a ModuleName],
        id: Id,
        side: Side,
        ink: Ink,
        unfolding: f32,
        deepest: usize
    ) -> Option<Element<'a, Message>> {
        let fan = self.fan_span();

        let blocks: Vec<Element<'a, Message>> = order
            .iter()
            .enumerate()
            .filter_map(|(within, unit)| {
                let (travel, bloom) =
                    hydebar_core::animation::share(unfolding, Self::reach(within, deepest));
                let block = self.desk_unit(unit, id, side, ink, bloom)?;

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
                )
                .departing_from(self.strip_row())
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

    /// How far the block standing `within` places down a column has to go.
    ///
    /// Against the block that goes furthest, which is the last place of the
    /// longest column: the places of a column are a row apart, so how many
    /// places down a block stands is how far down the screen it is bound.
    /// A block half as far down is there in half the time and opens then.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a layout holds a handful of units"
    )]
    pub(crate) fn reach(within: usize, deepest: usize) -> f32 {
        (within + 1) as f32 / (deepest.max(1)) as f32
    }

    /// The units of one section, in the order the canvas stands them in.
    ///
    /// A unit is one module. The modules of a group shared one pill on the
    /// strip and each carries a pill of its own to its place on the canvas:
    /// what parts is the icons, and the backing goes with both of them.
    ///
    /// The rule is the distance from the middle of the strip: a unit that
    /// stood near the centre stands high on the canvas, and one that stood at
    /// an edge reaches for the corner below it. The centre section already
    /// reads outwards from the middle; the left one reads towards it, so it
    /// is turned around.
    pub(crate) fn desk_order(
        section: &[ModuleDef],
        reads_towards_the_centre: bool
    ) -> Vec<&ModuleName> {
        let mut order: Vec<&ModuleName> = section.iter().flat_map(Self::members).collect();

        if reads_towards_the_centre {
            order.reverse();
        }

        order
    }

    /// How far inwards the nearest unit of a section stands.
    ///
    /// The units of a section fan out as they come down: the one that stood
    /// nearest the middle of the strip lands nearest the middle of the screen
    /// and the far one lands against the edge, each in a lane of its own.
    /// Falling straight down a single lane is what had them passing through
    /// one another — a block on its way to the fourth place crossed the three
    /// already standing.
    fn fan_span(&self) -> f32 {
        self.screen_width.unwrap_or(1920.0) * FAN
    }

    /// The height the strip's own islands stand at.
    ///
    /// Where every block departs from: the canvas covers the whole screen,
    /// strip band included, so the row the units leave is the top of the
    /// canvas rather than somewhere above it.
    fn strip_row(&self) -> f32 {
        self.appearance().bar_padding()[0]
    }

    /// Draws one unit in the form the canvas has room for.
    ///
    /// The opened block is built on every frame of the unfolding, empty of
    /// writing or full of it: it is what takes the unit's room in the column,
    /// and a unit that stood as a bare island until it began to open would
    /// take its room only then, moving everything below it down the screen
    /// mid-flight.
    fn desk_unit<'a>(
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
            Column::with_children(std::iter::once(island).chain(opened))
                .spacing(ink.size * 0.9)
                .width(Length::Fill)
                .align_x(side.alignment_x())
                .into()
        )
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
            return vec![self.desk_month()];
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

    /// The modules one unit carries.
    pub(crate) fn members(unit: &ModuleDef) -> Vec<&ModuleName> {
        match unit {
            ModuleDef::Single(module) => vec![module],
            ModuleDef::Group(group) => group.iter().collect()
        }
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
    /// straight onto the wallpaper instead of into a popup. It is there from
    /// the first frame of the unfolding: six rows arriving part way through
    /// would push the rest of the column down the screen at that moment, and
    /// the clock is flying at that moment.
    fn desk_month(&self) -> Element<'_, Message> {
        self.calendar.menu_view(self.icons()).map(Message::Calendar)
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
}
