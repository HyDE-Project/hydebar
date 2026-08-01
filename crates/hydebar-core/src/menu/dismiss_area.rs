//! Bar side of the rule that a press outside a menu dismisses it.
//!
//! The backdrop of a menu only covers the screen the bar leaves free, so a
//! press landing on the bar itself never reaches it. This wrapper reports
//! those presses instead, which is what lets the one rule hold over the whole
//! screen instead of stopping at the edge of the bar.

use iced::{
    Length, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget,
        event::Event,
        layout, mouse, renderer, touch,
        widget::{Operation, Tree, tree}
    }
};

/// Press state of a [`DismissArea`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct State {
    /// Whether the press in flight started on this area.
    pressed: bool
}

/// Wrapper reporting the presses that land on the element it wraps.
///
/// Unlike [`iced::widget::mouse_area`] it also reports a press a child widget
/// consumed: a module button swallowing its own press must not hide from the
/// bar that a press happened, or pressing that module would leave an open menu
/// behind.
///
/// The press and its completion are reported apart on purpose. A press only
/// arms the dismissal, and the module the press lands on is given the whole
/// click to take it over, so switching from one menu to the next never flashes
/// the menu surface off and on again.
pub struct DismissArea<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    content:    iced_core::Element<'a, Message, Theme, Renderer>,
    on_press:   Message,
    on_release: Message
}

impl<Message, Theme, Renderer> std::fmt::Debug for DismissArea<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DismissArea").finish_non_exhaustive()
    }
}

/// Wraps `content` so every press on it is reported.
///
/// `on_press` is published when a press lands on the wrapped element and
/// `on_release` once that same press completes on it.
pub fn dismiss_area<'a, Message, Theme, Renderer>(
    content: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>,
    on_press: Message,
    on_release: Message
) -> DismissArea<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    DismissArea {
        content: content.into(),
        on_press,
        on_release
    }
}

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
        let is_over = cursor.is_over(layout.bounds());

        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(_))
                | Event::Touch(touch::Event::FingerPressed { .. })
        ) && is_over
        {
            let state = tree.state.downcast_mut::<State>();
            state.pressed = true;
            shell.publish(self.on_press.clone());
        }

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
            Event::Mouse(mouse::Event::ButtonReleased(_))
            | Event::Touch(touch::Event::FingerLifted {
                ..
            }) => {
                if std::mem::take(&mut state.pressed) && is_over {
                    shell.publish(self.on_release.clone());
                }
            }
            Event::Mouse(mouse::Event::CursorLeft)
            | Event::Touch(touch::Event::FingerLost {
                ..
            }) => {
                state.pressed = false;
            }
            _ => {}
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

impl<'a, Message, Theme, Renderer> From<DismissArea<'a, Message, Theme, Renderer>>
    for iced_core::Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: iced_core::Renderer + 'a
{
    fn from(area: DismissArea<'a, Message, Theme, Renderer>) -> Self {
        Self::new(area)
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Theme, widget::Space};
    use iced_core::layout::Limits;

    use super::*;

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
