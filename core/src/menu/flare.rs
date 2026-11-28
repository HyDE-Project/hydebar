//! Concave flare drawn at the join between the bar and a drawer menu.
//!
//! The flare fills the square outside a quarter circle, producing the smooth
//! concave transition that makes the drawer look poured out of the bar. The
//! geometry is the same idea as Mandara's `concave_flare`, drawn with iced
//! canvas paths.

use iced::{
    Point, Rectangle, Renderer, Theme,
    mouse::Cursor,
    widget::canvas::{Frame, Geometry, Path, Program}
};

/// Which corner of the flare square the quarter circle is centered on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlareCorner {
    /// Circle center at top-left; fill hugs bottom-right.
    TopLeft,
    /// Circle center at top-right; fill hugs bottom-left.
    TopRight,
    /// Circle center at bottom-left; fill hugs top-right.
    BottomLeft,
    /// Circle center at bottom-right; fill hugs top-left.
    BottomRight
}

/// A concave flare: the `radius × radius` square minus the quarter circle
/// centered on `corner`.
#[derive(Debug, Clone, Copy)]
pub struct ConcaveFlare {
    /// Radius of the concave arc, in logical pixels.
    pub radius:  f32,
    /// The corner where the quarter circle is centered.
    pub corner:  FlareCorner,
    /// Opacity of the flare fill, matching the menu background.
    pub opacity: f32
}

impl<Message> Program<Message> for ConcaveFlare {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if self.radius <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let r = self.radius.min(bounds.width).min(bounds.height);
        let path = flare_path(r, self.corner);
        let color = theme.palette().background.scale_alpha(self.opacity);
        frame.fill(&path, color);

        vec![frame.into_geometry()]
    }
}

fn flare_path(r: f32, corner: FlareCorner) -> Path {
    Path::new(|builder| match corner {
        FlareCorner::TopLeft => {
            // Square (0,0)-(r,r), cut circle centered at (0,0) radius r.
            // Fill is bottom-right region.
            builder.move_to(Point::new(r, 0.0));
            builder.line_to(Point::new(r, r));
            builder.line_to(Point::new(0.0, r));
            builder.arc_to(Point::new(0.0, 0.0), Point::new(r, 0.0), r);
            builder.close();
        }
        FlareCorner::TopRight => {
            builder.move_to(Point::new(0.0, 0.0));
            builder.line_to(Point::new(0.0, r));
            builder.line_to(Point::new(r, r));
            builder.arc_to(Point::new(r, 0.0), Point::new(0.0, 0.0), r);
            builder.close();
        }
        FlareCorner::BottomLeft => {
            builder.move_to(Point::new(0.0, 0.0));
            builder.line_to(Point::new(r, 0.0));
            builder.line_to(Point::new(r, r));
            builder.arc_to(Point::new(0.0, r), Point::new(0.0, 0.0), r);
            builder.close();
        }
        FlareCorner::BottomRight => {
            builder.move_to(Point::new(r, 0.0));
            builder.line_to(Point::new(r, r));
            builder.line_to(Point::new(0.0, r));
            builder.arc_to(Point::new(r, r), Point::new(r, 0.0), r);
            builder.close();
        }
    })
}
