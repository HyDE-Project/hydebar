//! The shape one module takes once it is down on the canvas.

use hydebar_core::config::ModuleName;
use iced::{
    Element, Length, SurfaceId as Id,
    widget::{Column, container}
};

use super::super::{
    super::super::state::{App, Message},
    blocks::{self, Along, Ink, Side}
};

/// How far the light of a moving block reaches at its brightest, in pixels.
///
/// Wider than the shadow an island rests under and softer than its edge: what
/// is wanted is a block glowing, not a block with a second outline.
const HALO: f32 = 26.0;

/// How strong the light of a moving block is at its brightest.
///
/// Low. The glow says a block came loose or came to rest and then gets out of
/// the way; one bright enough to read as a colour of its own turns every
/// unfolding into a light show.
const GLEAM: f32 = 0.4;

/// A resting shadow, as much of it as the ground under it is still painted.
///
/// A shadow is cast by the thing above it: the pill fading out has to take
/// its own shadow with it, or the canvas is left with a shadow of nothing.
fn faded(resting: iced::Shadow, ground: f32) -> iced::Shadow {
    iced::Shadow {
        color: resting.color.scale_alpha(ground.clamp(0.0, 1.0)),
        ..resting
    }
}

/// The shadow a block wears while it is carrying `glow` of its journey's
/// light.
///
/// The island's own shadow opened out and lit, rather than a second thing
/// painted behind the pill: a block resting carries exactly what it always
/// carried, and one on the move carries the same shadow reaching further and
/// in the colour the theme lights things with.
fn halo(resting: iced::Shadow, lit: iced::Color, glow: f32) -> iced::Shadow {
    if glow <= 0.0 {
        return resting;
    }

    iced::Shadow {
        color:       iced::Color {
            a: GLEAM * glow,
            ..lit
        },
        offset:      iced::Vector::ZERO,
        blur_radius: HALO.mul_add(glow, resting.blur_radius)
    }
}

/// How wide a block is written, in body letters.
///
/// A reading is a label and a figure, and the eye pairs them by how close
/// they are. Given the whole third of a screen a column has, the label ends
/// up an arm's length from its figure and the pair stops being a pair — which
/// is why every table ever printed has a measure. This is that measure: wide
/// enough for the longest value the bar shows, narrow enough that the two
/// halves of a line still read as one line.
const MEASURE: f32 = 30.0;

/// How tall the strip stands when the appearance states no height of its own.
const ISLAND: f32 = 38.0;

impl App {
    /// Draws one unit in the form the canvas has room for.
    ///
    /// The opened block is built on every frame of the unfolding, empty of
    /// writing or full of it: it is what takes the unit's room in the column,
    /// and a unit that stood as a bare island until it began to open would
    /// take its room only then, moving everything below it down the screen
    /// mid-flight.
    ///
    /// What arrived as an island stays, and the pill around it does not. The
    /// pill is the strip's own shape — a readout with a bar's ground under it
    /// — and the canvas is the other shape of the same bar: pills left
    /// standing over the blocks they opened into gave the two shapes one look
    /// between them. The ground fades as the block writes itself out, so what
    /// the eye follows is one thing becoming another rather than a thing
    /// being swapped, and the reading itself is never taken away.
    pub(super) fn desk_unit<'a>(
        &'a self,
        unit: &'a ModuleName,
        id: Id,
        side: Side,
        ink: Ink,
        along: Along
    ) -> Option<Element<'a, Message>> {
        let island = self.desk_island(unit, id, along.glow, along.bloom)?;
        let island = self.awaiting_its_turn(island, along.travel);
        let opened: Vec<Element<'a, Message>> = self.desk_opened(unit, side, ink, along.bloom);

        if opened.is_empty() {
            return Some(island);
        }

        let named = Self::named(island, self.desk_heading(unit), side, ink, along.bloom);

        Some(
            container(
                Column::with_children(std::iter::once(named).chain(opened))
                    .spacing(ink.size * 0.9)
                    .width(Length::Fill)
                    .align_x(side.alignment_x())
            )
            .max_width(ink.size * MEASURE)
            .into()
        )
    }

    /// The name the first block of `unit` would have written over itself.
    ///
    /// Nothing when the unit opens into something that carries no name of its
    /// own — the month grid the clock opens into is the one — and the island
    /// then stands on its row alone.
    fn desk_heading(&self, unit: &ModuleName) -> Option<String> {
        if matches!(unit, ModuleName::Clock) {
            return None;
        }

        Some(
            self.desk_panels(unit)
                .first()
                .map_or_else(|| unit.label().to_owned(), |panel| panel.title.to_string())
        )
    }

    /// The island and the name of what it opens into, on one row.
    ///
    /// Two lines were three: the reading came in on its own row, the name of
    /// the block stood on the next one, and the rule under that. The island
    /// is what arrived, so the name belongs beside it.
    ///
    /// The name takes the column's own edge and the island stands inward of
    /// it. The islands are as wide as whatever they carry, so a name set
    /// after one lands wherever that island happened to end and the column
    /// loses the straight edge its names are read down.
    ///
    /// It is written in as its block opens, in step with the rule and the
    /// lines under it: the row a block arrives in is the island alone, and
    /// nothing of the arrival is spoiled by a name standing there before the
    /// block it names.
    fn named(
        island: Element<'_, Message>,
        heading: Option<String>,
        side: Side,
        ink: Ink,
        bloom: f32
    ) -> Element<'_, Message> {
        let Some(heading) = heading else {
            return island;
        };

        let written = blocks::name(&heading, ink, bloom);

        let row = match side {
            Side::Trailing => iced::widget::Row::with_children([island, written]),
            Side::Leading | Side::Middle => iced::widget::Row::with_children([written, island])
        };

        row.spacing(ink.size * 0.6)
            .align_y(iced::Alignment::Center)
            .into()
    }

    /// How tall an island stands on the strip, which is how tall it arrives.
    ///
    /// Not the height of the strip: the strip holds its islands inside its
    /// own padding, and an island given the whole band to stand in arrives a
    /// little taller than the one that left. Worked out the way the strip
    /// works it out, so the two cannot drift apart.
    fn island_height(&self) -> f32 {
        use hydebar_proto::config::AppearanceStyle;

        let appearance = self.appearance();
        let band = appearance.height_px().unwrap_or(ISLAND);

        if appearance.style == AppearanceStyle::Islands {
            appearance.bar_padding()[0].mul_add(-2.0, band)
        } else {
            band - 8.0
        }
    }

    /// The island of a unit that has not set off yet: its room, and nothing
    /// in it.
    ///
    /// The strip is still drawing this module, and two of it on one screen is
    /// the one frame this must not draw. Its place is held all the same, so
    /// the column is laid out from the first frame of the unfolding and
    /// nothing below it moves when its turn comes.
    fn awaiting_its_turn<'a>(
        &self,
        island: Element<'a, Message>,
        travel: f32
    ) -> Element<'a, Message> {
        if travel > 0.0 {
            return island;
        }

        iced::widget::Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(self.island_height()))
            .into()
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
        self.desk_island(unit, id, 0.0, 0.0).is_some()
    }

    /// The room one unit takes when it is open, at the given ink.
    ///
    /// Worked out from the same figures that reserve it, so what this says a
    /// column needs is what the column takes: the island it arrived as, the
    /// gap under it, and the room of every block it opens into.
    pub(in crate::app::view::desk) fn desk_unit_room(&self, unit: &ModuleName, ink: Ink) -> f32 {
        let island = self.island_height();

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
                ink.size.mul_add(0.9, blocks::room_of(&panel, ink, true))
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
            .enumerate()
            .map(|(place, panel)| blocks::room_of(panel, ink, place > 0))
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
                        .map(|panel| blocks::panel(&panel, side, ink, bloom, true))
                )
                .collect();
        }

        let panels = self.desk_panels(module);

        if panels.is_empty() {
            return vec![blocks::awaited(side, ink, bloom)];
        }

        panels
            .iter()
            .enumerate()
            .map(|(place, panel)| blocks::panel(panel, side, ink, bloom, place > 0))
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
    fn desk_island<'a>(
        &'a self,
        unit: &'a ModuleName,
        id: Id,
        glow: f32,
        bloom: f32
    ) -> Option<Element<'a, Message>> {
        let opacity = self.appearance().opacity;
        let (content, action) = self.get_module_view(unit, id, opacity)?;
        let actions = self.module_actions(unit, action);

        Some(self.desk_pill(
            vec![self.module_element(content, actions, unit, id, true)],
            glow,
            bloom
        ))
    }

    /// The ground a readout stands on while it is still the strip's.
    ///
    /// Painted in full while the block travels, gone by the time the block it
    /// heads is written out: what left the strip has to arrive looking like
    /// what left, and what stands on the canvas has to look like the canvas.
    /// `glow` is however much of its journey's light it is carrying.
    ///
    /// The room it takes never changes with any of that. The padding and the
    /// height are the strip's own, held whether the ground under them is
    /// painted or not: a pill that gave its padding back when it stopped
    /// being painted moved the readout standing in it, and a reading that
    /// jumps as its background fades is the fade drawing attention to itself.
    ///
    /// The height is the one the strip gives an island, so a block leaves the
    /// bar at the size it stood there and arrives at that size too.
    fn desk_pill<'a>(
        &'a self,
        members: Vec<Element<'a, Message>>,
        glow: f32,
        bloom: f32
    ) -> Element<'a, Message> {
        use hydebar_proto::config::AppearanceStyle;

        let appearance = self.appearance();
        let row = iced::widget::Row::with_children(members)
            .spacing(appearance.island_gap())
            .align_y(iced::Alignment::Center);

        let islands = appearance.style == AppearanceStyle::Islands;
        let glow = glow.clamp(0.0, 1.0);
        let ground = 1.0 - bloom.clamp(0.0, 1.0);
        let opacity = appearance.opacity;
        let finish = hydebar_core::style::IslandFinish::of(appearance);
        let radius = appearance.pill_radius();

        container(row)
            .height(Length::Fixed(self.island_height()))
            .align_y(iced::Alignment::Center)
            .padding(if islands {
                appearance.island_padding()
            } else {
                [0.0, 0.0]
            })
            .style(move |theme: &iced::Theme| iced::widget::container::Style {
                background: (islands && ground > 0.0).then(|| {
                    theme
                        .palette()
                        .background
                        .scale_alpha(opacity * ground)
                        .into()
                }),
                border: if islands {
                    let edge = finish.border(radius);

                    iced::Border {
                        color: edge.color.scale_alpha(ground),
                        ..edge
                    }
                } else {
                    iced::Border::default().rounded(radius)
                },
                shadow: halo(
                    faded(finish.shadow(), ground),
                    theme.palette().primary,
                    glow
                ),
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
