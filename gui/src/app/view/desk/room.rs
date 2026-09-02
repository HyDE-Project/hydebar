//! The measurements the canvas is laid out against.
//!
//! Where each column stands, how deep the deepest of them runs, and how much
//! screen is left below the strip for them to end within. Kept apart from the
//! drawing because every one of them is a fact about the screen rather than
//! about the frame: the drawing asks them and never argues with them.

use hydebar_core::config::ModuleName;
use iced::SurfaceId as Id;

use super::super::super::state::App;

/// The three sections of the layout, each in the order its column stands in.
pub(in crate::app::view) type Columns<'a> = (
    Vec<&'a ModuleName>,
    Vec<&'a ModuleName>,
    Vec<&'a ModuleName>
);

impl App {
    /// The three columns of the canvas, each in the order it stands in.
    ///
    /// Read from the edge of the screen inwards: the unit that stood nearest
    /// its own edge heads the column and the one that stood nearest the middle
    /// ends it. That is the one order in which no two journeys cross — every
    /// path falls, then closes in on the same edge, and paths laid this way
    /// nest inside one another instead of cutting across. Read the other way
    /// round, the near block's run to the edge cut straight through the far
    /// block's fall. Nothing here says when a unit moves — they all move at
    /// once — only where each of them is bound.
    pub(crate) fn desk_columns(modules: &hydebar_core::config::Modules) -> Columns<'_> {
        (
            Self::desk_order(&modules.left, false),
            Self::desk_order(&modules.center, false),
            Self::desk_order(&modules.right, true)
        )
    }

    /// How many places the longest column of the canvas stands.
    ///
    /// The measure every block's journey is stated against: the block at the
    /// bottom of this column is the one with the furthest to go.
    pub(crate) fn deepest_column(&self) -> usize {
        let (left, centre, right) = Self::desk_columns(&self.config.modules);

        left.len().max(centre.len()).max(right.len())
    }

    /// The height the columns have to end within.
    ///
    /// Everything below the strip's own band, less the margin the canvas keeps
    /// around itself, on a screen the bar has been told the height of. A
    /// screen it has not is answered with nothing to fit into, which leaves
    /// the writing at the size the theme asked for.
    pub(in crate::app::view::desk) fn canvas_room(&self, id: Id) -> f32 {
        let margin = self.appearance().font_size_px() * 2.0;

        self.screen_height
            .map_or(f32::INFINITY, |height| {
                height - self.strip_band(id) - margin * 2.0
            })
            .max(0.0)
    }

    /// The band along the top of the screen the strip itself occupies.
    ///
    /// The canvas covers the whole screen so its blocks can leave the strip
    /// without jumping, which means the places they land have to keep clear
    /// of the band the strip stands in — the strip's own height, and whatever
    /// reserved a band above it before the strip was put there.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the bar height constant is exactly representable in f32"
    )]
    pub(in crate::app::view::desk) fn strip_band(&self, id: Id) -> f32 {
        self.strip_top(id)
            + self
                .appearance()
                .height_px()
                .unwrap_or(hydebar_core::HEIGHT as f32)
    }
}
