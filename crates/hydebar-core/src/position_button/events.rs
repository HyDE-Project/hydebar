use iced::{
    Rectangle,
    core::{Clipboard, Layout, Shell, event::Event, mouse, touch, widget::Tree},
    widget::button::Catalog,
    window
};

use super::{
    builder::PositionButton,
    press::publish,
    state::{State, resolve_status}
};

/// Feeds an event to the button, dispatching the handler the pressed mouse
/// button carries and keeping the interaction state in sync.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors the upstream widget update signature"
)]
pub(super) fn update<Message, Theme, Renderer>(
    button: &mut PositionButton<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    event: &Event,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    renderer: &Renderer,
    clipboard: &mut dyn Clipboard,
    shell: &mut Shell<'_, Message>,
    viewport: &Rectangle
) where
    Message: Clone,
    Renderer: iced_core::Renderer,
    Theme: Catalog
{
    button.content.as_widget_mut().update(
        &mut tree.children[0],
        event,
        layout.children().next().unwrap(),
        cursor,
        renderer,
        clipboard,
        shell,
        viewport
    );

    match event {
        Event::Mouse(mouse::Event::ButtonPressed(pressed))
            if matches!(
                pressed,
                mouse::Button::Left | mouse::Button::Right | mouse::Button::Middle
            ) =>
        {
            if let Some(handler) = button.handler(*pressed)
                && cursor.is_over(layout.bounds())
            {
                let state = tree.state.downcast_mut::<State>();

                *state.hold_mut(*pressed) = true;
                publish(handler, layout, viewport, shell);
            }
        }
        Event::Touch(touch::Event::FingerPressed {
            ..
        }) => {
            if button.on_press.is_some() && cursor.is_over(layout.bounds()) {
                let state = tree.state.downcast_mut::<State>();

                state.is_pressed = true;
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(released))
            if matches!(
                released,
                mouse::Button::Left | mouse::Button::Right | mouse::Button::Middle
            ) =>
        {
            if button.handler(*released).is_some() {
                let state = tree.state.downcast_mut::<State>();

                let _ = std::mem::take(state.hold_mut(*released));
            }
        }
        Event::Touch(touch::Event::FingerLifted {
            ..
        }) => {
            if let Some(on_press) = button.on_press.as_ref() {
                let state = tree.state.downcast_mut::<State>();

                if std::mem::take(&mut state.is_pressed) && cursor.is_over(layout.bounds()) {
                    publish(on_press, layout, viewport, shell);
                }
            }
        }
        Event::Touch(touch::Event::FingerLost {
            ..
        })
        | Event::Mouse(mouse::Event::CursorLeft) => {
            let state = tree.state.downcast_mut::<State>();
            state.is_hovered = false;
            state.release_all();
        }
        _ => {}
    }

    let state = tree.state.downcast_mut::<State>();
    state.is_hovered = cursor.is_over(layout.bounds());

    let current_status =
        resolve_status(button.is_pressable(), state.is_hovered, state.is_pressed());

    if matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
        state.painted = Some(current_status);
    } else if state
        .painted
        .is_some_and(|painted| painted != current_status)
    {
        shell.request_redraw();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use iced::{
        Point, Size, Theme,
        core::{Widget, clipboard, layout, layout::Limits, window::RedrawRequest},
        widget::Space
    };

    use super::*;
    use crate::position_button::position_button;

    type TestRenderer = ();

    const BOUNDS: Rectangle = Rectangle {
        x:      0.0,
        y:      0.0,
        width:  36.0,
        height: 26.0
    };

    struct Harness<'a> {
        button: PositionButton<'a, (), Theme, TestRenderer>,
        tree:   Tree,
        node:   layout::Node
    }

    fn button<'a>(pressable: bool) -> PositionButton<'a, (), Theme, TestRenderer> {
        let button = position_button(Space::new().width(16.0).height(16.0));

        if pressable {
            button.on_press(())
        } else {
            button
        }
    }

    fn harness<'a>(pressable: bool) -> Harness<'a> {
        let mut tree = Tree::new(iced_core::Element::<(), Theme, TestRenderer>::new(button(
            pressable
        )));
        let mut button = button(pressable);

        let node = button.layout(
            &mut tree,
            &(),
            &Limits::new(Size::ZERO, Size::new(200.0, 26.0))
        );

        Harness {
            button,
            tree,
            node
        }
    }

    fn feed(harness: &mut Harness<'_>, event: &Event, cursor: mouse::Cursor) -> RedrawRequest {
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let mut clipboard = clipboard::Null;

        harness.button.update(
            &mut harness.tree,
            event,
            Layout::new(&harness.node),
            cursor,
            &(),
            &mut clipboard,
            &mut shell,
            &BOUNDS
        );

        shell.redraw_request()
    }

    fn redraw_event() -> Event {
        Event::Window(iced::window::Event::RedrawRequested(Instant::now()))
    }

    fn moved_to(x: f32) -> Event {
        Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(x, 10.0)
        })
    }

    fn cursor_at(x: f32) -> mouse::Cursor {
        mouse::Cursor::Available(Point::new(x, 10.0))
    }

    #[test]
    fn entering_the_button_asks_the_runtime_for_a_frame() {
        let mut harness = harness(true);

        // the painted frame knows the cursor sitting outside the button
        feed(&mut harness, &redraw_event(), cursor_at(500.0));

        assert_eq!(
            feed(&mut harness, &moved_to(10.0), cursor_at(10.0)),
            RedrawRequest::NextFrame,
            "a hover the painted frame does not show has to schedule a redraw"
        );
    }

    #[test]
    fn leaving_the_button_asks_the_runtime_for_a_frame() {
        let mut harness = harness(true);

        feed(&mut harness, &redraw_event(), cursor_at(10.0));

        assert_eq!(
            feed(&mut harness, &moved_to(500.0), cursor_at(500.0)),
            RedrawRequest::NextFrame
        );
    }

    #[test]
    fn moving_inside_the_button_does_not_ask_for_a_frame() {
        let mut harness = harness(true);

        feed(&mut harness, &redraw_event(), cursor_at(10.0));

        assert_eq!(
            feed(&mut harness, &moved_to(20.0), cursor_at(20.0)),
            RedrawRequest::Wait,
            "the painted frame already shows the hover"
        );
    }

    #[test]
    fn an_inert_button_never_asks_for_a_frame() {
        let mut harness = harness(false);

        feed(&mut harness, &redraw_event(), cursor_at(500.0));

        assert_eq!(
            feed(&mut harness, &moved_to(10.0), cursor_at(10.0)),
            RedrawRequest::Wait
        );
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Pressed {
        Left,
        Right,
        Middle
    }

    struct ButtonHarness<'a> {
        button: PositionButton<'a, Pressed, Theme, TestRenderer>,
        tree:   Tree,
        node:   layout::Node
    }

    fn every_button<'a>() -> PositionButton<'a, Pressed, Theme, TestRenderer> {
        position_button(Space::new().width(16.0).height(16.0))
            .on_press(Pressed::Left)
            .on_right_press(Pressed::Right)
            .on_middle_press(Pressed::Middle)
    }

    fn button_harness<'a>() -> ButtonHarness<'a> {
        let mut tree = Tree::new(iced_core::Element::<Pressed, Theme, TestRenderer>::new(
            every_button()
        ));
        let mut button = every_button();

        let node = button.layout(
            &mut tree,
            &(),
            &Limits::new(Size::ZERO, Size::new(200.0, 26.0))
        );

        ButtonHarness {
            button,
            tree,
            node
        }
    }

    fn press(harness: &mut ButtonHarness<'_>, button: mouse::Button) -> Vec<Pressed> {
        let mut messages = Vec::new();

        for event in [
            Event::Mouse(mouse::Event::ButtonPressed(button)),
            Event::Mouse(mouse::Event::ButtonReleased(button))
        ] {
            let mut shell = Shell::new(&mut messages);
            let mut clipboard = clipboard::Null;

            harness.button.update(
                &mut harness.tree,
                &event,
                Layout::new(&harness.node),
                cursor_at(10.0),
                &(),
                &mut clipboard,
                &mut shell,
                &BOUNDS
            );
        }

        messages
    }

    #[test]
    fn dispatches_the_handler_of_the_pressed_mouse_button() {
        let mut harness = button_harness();

        assert_eq!(
            press(&mut harness, mouse::Button::Right),
            vec![Pressed::Right]
        );
        assert_eq!(
            press(&mut harness, mouse::Button::Middle),
            vec![Pressed::Middle]
        );
        assert_eq!(
            press(&mut harness, mouse::Button::Left),
            vec![Pressed::Left]
        );
    }

    #[test]
    fn ignores_a_mouse_button_without_a_handler() {
        let mut tree = Tree::new(iced_core::Element::<Pressed, Theme, TestRenderer>::new(
            position_button(Space::new().width(16.0).height(16.0)).on_press(Pressed::Left)
        ));
        let mut button: PositionButton<'_, Pressed, Theme, TestRenderer> =
            position_button(Space::new().width(16.0).height(16.0)).on_press(Pressed::Left);

        let node = button.layout(
            &mut tree,
            &(),
            &Limits::new(Size::ZERO, Size::new(200.0, 26.0))
        );

        let mut harness = ButtonHarness {
            button,
            tree,
            node
        };

        assert!(press(&mut harness, mouse::Button::Right).is_empty());
        assert_eq!(
            press(&mut harness, mouse::Button::Left),
            vec![Pressed::Left]
        );
    }
}
