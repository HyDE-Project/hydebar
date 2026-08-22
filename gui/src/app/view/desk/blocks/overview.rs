//! The workspaces of a screen, drawn as the rooms they are.
//!
//! A list saying "three windows" is a fact about a workspace; a miniature of
//! it is the workspace itself, and the eye picks the one it wants out of a row
//! of them without reading a word. Every screen that ever offered an overview
//! drew it this way, and the compositor already knows where every window
//! stands, so nothing is guessed here — the shapes are the layout, to scale.

use iced::{
    Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme,
    mouse::Cursor,
    widget::{
        Column, Row,
        canvas::{self, Canvas, Frame as Sheet, Geometry, Path, Stroke},
        text
    }
};

use super::{super::readings::Miniature, Ink};
use crate::app::Message;

/// How tall a miniature stands, as a share of the body ink.
///
/// A little over three lines: small enough that a column of them still reads
/// as one block, big enough that a window standing in a corner is a shape
/// rather than a speck.
const HEIGHT: f32 = 3.4;

/// Aspect of the miniature, wider than tall the way a screen is.
const ASPECT: f32 = 16.0 / 9.0;

/// Radius the corners of a window are rounded by, in miniature pixels.
const CORNER: f32 = 2.0;

/// Gap kept between two miniatures, as a share of the body ink.
const GAP: f32 = 0.5;

/// How tall the caption under a miniature stands, as a share of the body ink.
const CAPTION: f32 = 1.4;

/// The room a row of miniatures takes, at the given ink.
///
/// The miniature and the name under it, with the gap the column keeps between
/// two lines between them.
pub(super) fn room(ink: Ink) -> f32 {
    ink.size.mul_add(HEIGHT + CAPTION, ink.size * 0.28)
}

/// Draws one miniature per workspace, side by side.
pub(super) fn overview<'a>(workspaces: &[Miniature], ink: Ink) -> Element<'a, Message> {
    let height = ink.size * HEIGHT;

    Row::with_children(workspaces.iter().map(|workspace| {
        let name = text(workspace.name.clone())
            .size(ink.size * 0.9)
            .color(if workspace.active {
                ink.value
            } else {
                ink.label()
            });

        Column::new()
            .push(
                Canvas::new(Screen {
                    workspace: workspace.clone(),
                    ink
                })
                .width(Length::Fixed(height * ASPECT))
                .height(Length::Fixed(height))
            )
            .push(name)
            .spacing(ink.size * 0.28)
            .align_x(Alignment::Center)
            .into()
    }))
    .spacing(ink.size * GAP)
    .into()
}

/// One workspace, drawn to the size the row gave it.
#[derive(Debug)]
struct Screen {
    /// The workspace and the windows standing on it.
    workspace: Miniature,
    /// What the column is written in, which is what this is drawn in.
    ink:       Ink
}

impl canvas::Program<Message> for Screen {
    type State = ();

    fn draw(
        &self,
        (): &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor
    ) -> Vec<Geometry> {
        let mut sheet = Sheet::new(renderer, bounds.size());
        let ground = self
            .ink
            .value
            .scale_alpha(if self.workspace.active { 0.16 } else { 0.07 });

        sheet.fill(
            &Path::rounded_rectangle(Point::ORIGIN, bounds.size(), CORNER.into()),
            ground
        );

        for window in &self.workspace.windows {
            paint(&mut sheet, bounds.size(), window, self.ink.value);
        }

        sheet.stroke(
            &Path::rounded_rectangle(Point::ORIGIN, bounds.size(), CORNER.into()),
            Stroke::default().with_width(1.0).with_color(self.edge())
        );

        vec![sheet.into_geometry()]
    }
}

impl Screen {
    /// The colour the miniature is outlined in.
    ///
    /// The one on screen is stated plainly, one asking for attention more
    /// plainly still, and the rest are barely there: a row of equally drawn
    /// boxes says nothing about which of them the user is standing in.
    fn edge(&self) -> Color {
        if self.workspace.urgent {
            return self.ink.value.scale_alpha(0.9);
        }

        self.ink
            .value
            .scale_alpha(if self.workspace.active { 0.7 } else { 0.2 })
    }
}

/// Paints one window into the miniature, in the place it stands on the screen.
fn paint(sheet: &mut Sheet, screen: Size, window: &super::super::readings::Frame, ink: Color) {
    let at = Point::new(window.x * screen.width, window.y * screen.height);
    let size = Size::new(
        (window.width * screen.width).max(2.0),
        (window.height * screen.height).max(2.0)
    );

    let alpha = match (window.focused, window.floating) {
        (true, _) => 0.75,
        (false, true) => 0.45,
        (false, false) => 0.33
    };

    sheet.fill(
        &Path::rounded_rectangle(at, size, CORNER.into()),
        ink.scale_alpha(alpha)
    );
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    fn ink() -> Ink {
        Ink {
            value: Color::WHITE,
            size:  14.0
        }
    }

    #[test]
    fn a_miniature_is_as_tall_as_the_room_it_asks_for() {
        assert_eq!(room(ink()), 14.0 * HEIGHT);
    }

    #[test]
    fn the_workspace_in_view_is_outlined_more_plainly_than_the_rest() {
        let screen = |active: bool, urgent: bool| Screen {
            workspace: Miniature {
                name: "1".to_owned(),
                active,
                urgent,
                windows: Vec::new()
            },
            ink:       ink()
        };

        assert!(screen(true, false).edge().a > screen(false, false).edge().a);
        assert!(screen(false, true).edge().a > screen(true, false).edge().a);
    }
}
