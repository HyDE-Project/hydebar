//! How the tooltip anchor behaves inside the widget tree.
//!
//! Layout, drawing and interaction all pass straight through to the wrapped
//! module; the anchor's own work happens in `update`, where the hover is
//! compared against what was last published and the change — entry with the
//! element's placement, exit with [`None`] — is sent up.

use iced::{
    Length, Point, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget,
        event::Event,
        layout, mouse, renderer, touch,
        widget::{Operation, Tree, tree}
    }
};

use super::element::{State, TooltipAnchor};
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

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TooltipAnchor<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: 'a + iced_core::Renderer
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport
        );

        let state = tree.state.downcast_mut::<State>();

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

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector
    ) -> Option<iced_core::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation
        )
    }
}

#[cfg(test)]
mod tests {
    use iced::{Theme, widget::Space};
    use iced_core::layout::Limits;

    use super::*;
    use crate::tooltip::anchor::tooltip_anchor;

    type TestRenderer = ();

    const VIEWPORT: Rectangle = Rectangle {
        x:      0.0,
        y:      0.0,
        width:  1360.0,
        height: 38.0
    };

    struct Harness<'a> {
        anchor: TooltipAnchor<'a, Option<ButtonUIRef>, Theme, TestRenderer>,
        tree:   Tree,
        node:   layout::Node
    }

    fn harness<'a>() -> Harness<'a> {
        let build = || {
            tooltip_anchor::<Option<ButtonUIRef>, Theme, TestRenderer>(
                Space::new().width(40.0).height(30.0),
                |anchor| anchor
            )
        };

        let mut tree = Tree::new(
            iced_core::Element::<Option<ButtonUIRef>, Theme, TestRenderer>::new(build())
        );
        let mut anchor = build();
        let node = anchor.layout(
            &mut tree,
            &(),
            &Limits::new(Size::ZERO, Size::new(1360.0, 38.0))
        );

        Harness {
            anchor,
            tree,
            node
        }
    }

    fn feed(harness: &mut Harness<'_>, cursor: mouse::Cursor) -> Vec<Option<ButtonUIRef>> {
        let event = Event::Mouse(mouse::Event::CursorMoved {
            position: cursor.position().unwrap_or(Point::new(-1.0, -1.0))
        });

        feed_event(harness, &event, cursor)
    }

    fn feed_event(
        harness: &mut Harness<'_>,
        event: &Event,
        cursor: mouse::Cursor
    ) -> Vec<Option<ButtonUIRef>> {
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let mut clipboard = iced_core::clipboard::Null;

        harness.anchor.update(
            &mut harness.tree,
            event,
            Layout::new(&harness.node),
            cursor,
            &(),
            &mut clipboard,
            &mut shell,
            &VIEWPORT
        );

        messages
    }

    #[test]
    fn entering_the_module_publishes_its_placement() {
        let mut harness = harness();

        let messages = feed(
            &mut harness,
            mouse::Cursor::Available(Point::new(10.0, 10.0))
        );

        assert_eq!(
            messages,
            vec![Some(ButtonUIRef {
                position: Point::new(20.0, 15.0),
                viewport: (1360.0, 38.0)
            })]
        );
    }

    #[test]
    fn staying_inside_the_module_publishes_nothing_new() {
        let mut harness = harness();
        let _ = feed(
            &mut harness,
            mouse::Cursor::Available(Point::new(10.0, 10.0))
        );

        assert!(
            feed(
                &mut harness,
                mouse::Cursor::Available(Point::new(20.0, 10.0))
            )
            .is_empty(),
            "a tooltip already shown must not be republished on every move"
        );
    }

    #[test]
    fn leaving_the_module_clears_the_tooltip() {
        let mut harness = harness();
        let _ = feed(
            &mut harness,
            mouse::Cursor::Available(Point::new(10.0, 10.0))
        );

        assert_eq!(
            feed(
                &mut harness,
                mouse::Cursor::Available(Point::new(500.0, 10.0))
            ),
            vec![None]
        );
    }

    #[test]
    fn leaving_the_bar_clears_the_tooltip_for_good() {
        let mut harness = harness();
        let inside = mouse::Cursor::Available(Point::new(10.0, 10.0));
        let _ = feed(&mut harness, inside);

        let left = Event::Mouse(mouse::Event::CursorLeft);
        assert_eq!(feed_event(&mut harness, &left, inside), vec![None]);

        // the surface keeps reporting the position the pointer had when it
        // left, so the next frame must not read it as a hover all over again
        let redraw = Event::Window(iced::window::Event::RedrawRequested(
            std::time::Instant::now()
        ));

        assert!(feed_event(&mut harness, &redraw, inside).is_empty());
    }
}
