//! How the dismiss area behaves inside the widget tree.
//!
//! Layout, drawing and interaction pass straight through to the wrapped
//! content; the area's own work is in [`press`], which is given the event
//! before the children and again after them.

mod press;

use iced::{
    Length, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget,
        event::Event,
        layout, mouse, renderer,
        widget::{Operation, Tree, tree}
    }
};

use super::element::{DismissArea, State};

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for DismissArea<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
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
        let is_over = Self::is_over(layout, cursor);

        self.note_the_press(tree.state.downcast_mut::<State>(), event, is_over, shell);

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

        self.note_the_release(tree.state.downcast_mut::<State>(), event, is_over, shell);
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use iced::{Point, Theme, widget::Space};
    use iced_core::layout::Limits;

    use super::*;
    use crate::menu::dismiss_area::dismiss_area;

    type TestRenderer = ();

    const VIEWPORT: Rectangle = Rectangle {
        x:      0.0,
        y:      0.0,
        width:  1360.0,
        height: 38.0
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Reported {
        Pressed,
        Released
    }

    struct Harness<'a> {
        area: DismissArea<'a, Reported, Theme, TestRenderer>,
        tree: Tree,
        node: layout::Node
    }

    fn harness<'a>() -> Harness<'a> {
        let build = || {
            dismiss_area::<Reported, Theme, TestRenderer>(
                Space::new().width(100.0).height(30.0),
                Reported::Pressed,
                Reported::Released
            )
        };

        let mut tree = Tree::new(iced_core::Element::<Reported, Theme, TestRenderer>::new(
            build()
        ));
        let mut area = build();
        let node = area.layout(
            &mut tree,
            &(),
            &Limits::new(Size::ZERO, Size::new(1360.0, 38.0))
        );

        Harness {
            area,
            tree,
            node
        }
    }

    fn cursor_at(x: f32) -> mouse::Cursor {
        mouse::Cursor::Available(Point::new(x, 10.0))
    }

    fn feed(harness: &mut Harness<'_>, event: &Event, cursor: mouse::Cursor) -> Vec<Reported> {
        feed_captured(harness, event, cursor, false)
    }

    fn feed_captured(
        harness: &mut Harness<'_>,
        event: &Event,
        cursor: mouse::Cursor,
        captured: bool
    ) -> Vec<Reported> {
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let mut clipboard = iced_core::clipboard::Null;

        if captured {
            shell.capture_event();
        }

        harness.area.update(
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

    fn pressed() -> Event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
    }

    fn released() -> Event {
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
    }

    #[test]
    fn a_click_reports_its_press_and_its_completion() {
        let mut harness = harness();

        assert_eq!(
            feed(&mut harness, &pressed(), cursor_at(10.0)),
            vec![Reported::Pressed]
        );
        assert_eq!(
            feed(&mut harness, &released(), cursor_at(10.0)),
            vec![Reported::Released]
        );
    }

    #[test]
    fn a_press_a_child_consumed_is_reported_all_the_same() {
        let mut harness = harness();

        // a module button swallows its own press, and the bar still has to
        // learn that a press happened outside the open menu
        assert_eq!(
            feed_captured(&mut harness, &pressed(), cursor_at(10.0), true),
            vec![Reported::Pressed]
        );
        assert_eq!(
            feed_captured(&mut harness, &released(), cursor_at(10.0), true),
            vec![Reported::Released]
        );
    }

    #[test]
    fn a_press_landing_elsewhere_is_ignored() {
        let mut harness = harness();

        assert!(feed(&mut harness, &pressed(), cursor_at(500.0)).is_empty());
        assert!(feed(&mut harness, &released(), cursor_at(500.0)).is_empty());
    }

    #[test]
    fn a_release_without_its_press_reports_nothing() {
        let mut harness = harness();

        assert!(feed(&mut harness, &released(), cursor_at(10.0)).is_empty());
    }

    #[test]
    fn a_press_dragged_off_the_area_never_completes() {
        let mut harness = harness();
        let _ = feed(&mut harness, &pressed(), cursor_at(10.0));

        assert!(feed(&mut harness, &released(), cursor_at(500.0)).is_empty());
    }

    #[test]
    fn leaving_the_surface_forgets_the_press_in_flight() {
        let mut harness = harness();
        let _ = feed(&mut harness, &pressed(), cursor_at(10.0));

        let _ = feed(
            &mut harness,
            &Event::Mouse(mouse::Event::CursorLeft),
            cursor_at(10.0)
        );

        assert!(feed(&mut harness, &released(), cursor_at(10.0)).is_empty());
    }
}
