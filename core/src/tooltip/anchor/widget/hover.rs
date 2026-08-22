//! Reading the hover off one event, and telling it apart from the last one.
//!
//! All the anchor's own work: layout, drawing and interaction pass straight
//! through to the wrapped module, and what is left is deciding whether the
//! pointer is on the module now and publishing the change — entry with the
//! element's placement, exit with [`None`].

use iced::{
    Point, Rectangle,
    core::{Layout, Shell, event::Event, mouse, touch}
};

use super::super::element::{State, TooltipAnchor};
use crate::position_button::ButtonUIRef;

/// Describes the wrapped element the way a menu describes its button.
fn anchor_of(layout: Layout<'_>, viewport: &Rectangle) -> ButtonUIRef {
    ButtonUIRef {
        position: Point::new(
            layout.bounds().width / 2. + layout.position().x,
            layout.bounds().height / 2. + layout.position().y
        ),
        viewport: (viewport.width, viewport.height)
    }
}

impl<'a, Message, Theme, Renderer> TooltipAnchor<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: 'a + iced_core::Renderer
{
    /// Answers one event with the hover it leaves behind, if it changed one.
    pub(super) fn note_the_hover(
        &self,
        state: &mut State,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle
    ) {
        match event {
            Event::Mouse(mouse::Event::CursorLeft)
            | Event::Touch(touch::Event::FingerLost {
                ..
            }) => state.pointer_inside = false,
            Event::Mouse(
                mouse::Event::CursorEntered
                | mouse::Event::CursorMoved {
                    ..
                }
            ) => state.pointer_inside = true,
            _ => {}
        }

        let is_hovered = state.pointer_inside && cursor.is_over(layout.bounds());

        if state.is_hovered != is_hovered {
            state.is_hovered = is_hovered;

            shell.publish((self.on_hover)(
                is_hovered.then(|| anchor_of(layout, viewport))
            ));
        }
    }
}
