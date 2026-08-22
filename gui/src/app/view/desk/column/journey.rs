//! Where a block leaves from, and how far it has to go.

use iced::SurfaceId as Id;

use super::super::super::super::state::App;

/// The share of the screen the fan of one section reaches across.
///
/// Nothing: the side sections stand on the edges of the screen they belong
/// to, every block of them on the same line, so a column has an edge for the
/// eye to run down. The lanes a fan once gave them are not what keeps two
/// blocks apart in flight — a block falls out of the strip's row before it
/// closes in, and it is the level between them that does — so the fan buys
/// nothing and costs the column its edge.
const FAN: f32 = 0.0;

/// The share of the longest journey the block nearest the strip travels.
///
/// A block one row down has less way to go than one at the bottom corner, but
/// not a seventh as much: the strip is a row above the first place, not at
/// it. Below this the near blocks are over before the eye has them, which
/// reads as appearing rather than arriving.
const NEAREST: f32 = 0.4;

impl App {
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
        if deepest < 2 {
            return 1.0;
        }

        let depth = (within as f32 / (deepest - 1) as f32).clamp(0.0, 1.0);

        (1.0 - NEAREST).mul_add(depth, NEAREST)
    }

    /// How far inwards the nearest unit of a section stands.
    ///
    /// The units of a section fan out as they come down: the one that stood
    /// nearest the middle of the strip lands nearest the middle of the screen
    /// and the far one lands against the edge, each in a lane of its own.
    /// Falling straight down a single lane is what had them passing through
    /// one another — a block on its way to the fourth place crossed the three
    /// already standing.
    pub(super) fn fan_span(&self) -> f32 {
        self.screen_width.unwrap_or(1920.0) * FAN
    }

    /// The height the strip's own islands stand at.
    ///
    /// Where every block departs from: the canvas covers the whole screen,
    /// strip band included, so the row the units leave is a row of the canvas
    /// rather than somewhere above it.
    ///
    /// The strip is not at the top of the screen because it is a bar. It is
    /// wherever the compositor put it, under whatever else reserved a band
    /// above it, and a layer surface is never told its own place — so the row
    /// is the measured one, with the padding the islands sit at inside the
    /// strip on top of it. Taken as zero, every block leapt the height of
    /// whatever stands above the bar before it began to move.
    pub(in crate::app::view::desk) fn strip_row(&self, id: Id) -> f32 {
        self.strip_top(id) + self.appearance().bar_padding()[0]
    }

    /// Where the strip's own surface begins on the screen it is drawn on.
    ///
    /// Zero until the compositor has answered, which is what a strip with
    /// nothing above it stands at anyway.
    pub(crate) fn strip_top(&self, id: Id) -> f32 {
        self.outputs
            .screen_of(id)
            .flatten()
            .and_then(|screen| self.strip_rows.get(screen))
            .copied()
            .unwrap_or_default()
    }
}
