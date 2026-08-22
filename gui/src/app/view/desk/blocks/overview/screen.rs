//! One workspace, drawn as the wallpaper it stands on and the windows on it.

use iced::{
    Color, Element, Length, Point, Rectangle, Renderer, Size, Theme,
    mouse::Cursor,
    widget::{
        Stack,
        canvas::{self, Canvas, Frame as Sheet, Geometry, Path, Stroke},
        image
    }
};

use super::super::super::readings::Miniature;
use crate::app::{Message, view::desk::blocks::Ink};

/// Radius the corners of a miniature and of a window are rounded by.
const CORNER: f32 = 2.0;

/// How plainly the wallpaper is drawn under the workspace in view.
const GROUND: f32 = 0.9;

/// How plainly it is drawn under one that is not.
const FAINT: f32 = 0.4;

/// Draws one workspace at the size the row gave it.
///
/// The wallpaper goes under the window shapes where the bar has read it: the
/// shapes alone say how the workspace is laid out, and the shapes over the
/// desktop's own picture say which desktop it is — which is the difference
/// between a diagram of a workspace and a preview of one.
pub(super) fn drawn<'a>(
    workspace: &Miniature,
    ground: Option<&iced::widget::image::Handle>,
    wide: f32,
    tall: f32,
    ink: Ink
) -> Element<'a, Message> {
    let rooms = Canvas::new(Screen {
        workspace: workspace.clone(),
        washed: ground.is_some(),
        ink
    })
    .width(Length::Fixed(wide))
    .height(Length::Fixed(tall));

    let Some(picture) = ground else {
        return rooms.into();
    };

    Stack::new()
        .push(
            image(picture.clone())
                .width(Length::Fixed(wide))
                .height(Length::Fixed(tall))
                .content_fit(iced::ContentFit::Cover)
                .opacity(if workspace.active { GROUND } else { FAINT })
        )
        .push(rooms)
        .into()
}

/// One workspace, drawn to the size the row gave it.
#[derive(Debug)]
struct Screen {
    /// The workspace and the windows standing on it.
    workspace: Miniature,
    /// Whether the wallpaper is drawn underneath.
    washed:    bool,
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

        sheet.fill(
            &Path::rounded_rectangle(Point::ORIGIN, bounds.size(), CORNER.into()),
            self.ground()
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
    /// The wash the windows are drawn over.
    ///
    /// Barely there over a wallpaper and a plain surface without one: a wash
    /// heavy enough to stand on its own would grey out the very picture it is
    /// meant to let through.
    fn ground(&self) -> Color {
        let share = match (self.washed, self.workspace.active) {
            (true, true) => 0.04,
            (true, false) => 0.06,
            (false, true) => 0.16,
            (false, false) => 0.07
        };

        self.ink.value.scale_alpha(share)
    }

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
///
/// Filled and outlined both: a wallpaper is a photograph, and a shape washed
/// over it in one flat tint disappears wherever the photograph happens to be
/// as bright as the tint. The outline is what keeps a window a window over any
/// picture at all.
fn paint(
    sheet: &mut Sheet,
    screen: Size,
    window: &super::super::super::readings::Frame,
    ink: Color
) {
    let at = Point::new(window.x * screen.width, window.y * screen.height);
    let size = Size::new(
        (window.width * screen.width).max(2.0),
        (window.height * screen.height).max(2.0)
    );

    let alpha = match (window.focused, window.floating) {
        (true, _) => 0.7,
        (false, true) => 0.5,
        (false, false) => 0.42
    };
    let shape = Path::rounded_rectangle(at, size, CORNER.into());

    sheet.fill(&shape, ink.scale_alpha(alpha));
    sheet.stroke(
        &shape,
        Stroke::default()
            .with_width(1.0)
            .with_color(ink.scale_alpha(0.85))
    );
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn screen(active: bool, urgent: bool, washed: bool) -> Screen {
        Screen {
            workspace: Miniature {
                name: "1".to_owned(),
                active,
                urgent,
                windows: Vec::new()
            },
            washed,
            ink: Ink {
                value: Color::WHITE,
                size:  14.0
            }
        }
    }

    #[test]
    fn the_workspace_in_view_is_outlined_more_plainly_than_the_rest() {
        assert!(screen(true, false, false).edge().a > screen(false, false, false).edge().a);
        assert!(screen(false, true, false).edge().a > screen(true, false, false).edge().a);
    }

    /// The wallpaper is the preview; a wash that hid it would undo the point.
    #[test]
    fn the_wash_stands_back_where_a_wallpaper_is_drawn_under_it() {
        assert!(screen(true, false, true).ground().a < screen(true, false, false).ground().a);
        assert!(screen(false, false, true).ground().a < screen(false, false, false).ground().a);
    }
}
