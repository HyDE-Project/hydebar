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
    /// carries, and nothing else moves.
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
        order: &[(usize, &'a ModuleName)],
        id: Id,
        side: Side,
        ink: Ink,
        unfolding: f32,
        units: usize
    ) -> Option<Element<'a, Message>> {
        let fan = self.fan_span();

        #[expect(
            clippy::cast_precision_loss,
            reason = "a layout holds a handful of units, far below any precision limit"
        )]
        let last = units.saturating_sub(1) as f32;

        let blocks: Vec<Element<'a, Message>> = order
            .iter()
            .enumerate()
            .filter_map(|(within, (turn, unit))| {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a layout holds a handful of units"
                )]
                let place = if last > 0.0 { *turn as f32 / last } else { 0.0 };
                let (travel, bloom) = hydebar_core::animation::share(unfolding, place, units);

                if travel <= 0.0 {
                    return None;
                }

                let block = self.desk_unit(unit, id, side, ink, bloom)?;
                let inwards = if last > 0.0 {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "a section holds a handful of units"
                    )]
                    let depth = within as f32 / (order.len().max(2) - 1) as f32;

                    fan * (1.0 - depth)
                } else {
                    0.0
                };

                let anchor = hydebar_core::components::flip::FlipAnchor::new(
                    self.flip_key(unit, id),
                    travel,
                    &self.flip,
                    block
                )
                .departing_from(self.strip_row());

                let travelling: Element<'a, Message> = if side == Side::Middle {
                    anchor.into()
                } else {
                    anchor.descending_first().into()
                };

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
    fn desk_unit<'a>(
        &'a self,
        unit: &'a ModuleName,
        id: Id,
        side: Side,
        ink: Ink,
        bloom: f32
    ) -> Option<Element<'a, Message>> {
        let island = self.desk_island(unit, id)?;

        if bloom <= 0.0 {
            return Some(island);
        }

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
            return self.desk_month(bloom).into_iter().collect();
        }

        self.desk_panels(module)
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
