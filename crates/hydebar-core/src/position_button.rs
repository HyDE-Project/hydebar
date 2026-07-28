use iced::{
    Background, Color, Element, Length, Padding, Point, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget,
        event::Event,
        keyboard, layout, mouse, overlay, renderer, touch,
        widget::{Operation, Tree, tree}
    },
    id::Id,
    widget::button::{Catalog, Status, Style, StyleFn},
    window
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonUIRef {
    pub position: Point,
    pub viewport: (f32, f32)
}

impl Eq for ButtonUIRef {}

enum OnPress<'a, Message> {
    Message(Message),
    MessageWithPosition(Box<dyn Fn(ButtonUIRef) -> Message + 'a>)
}

pub struct PositionButton<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::core::Renderer,
    Theme: Catalog
{
    content:  Element<'a, Message, Theme, Renderer>,
    on_press: Option<OnPress<'a, Message>>,
    id:       Id,
    width:    Length,
    height:   Length,
    padding:  Padding,
    clip:     bool,
    class:    Theme::Class<'a>
}

impl<'a, Message, Theme, Renderer> PositionButton<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
    Theme: Catalog
{
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        let content = content.into();
        let size = content.as_widget().size_hint();

        PositionButton {
            content,
            id: Id::unique(),
            on_press: None,
            width: size.width.fluid(),
            height: size.height.fluid(),
            padding: DEFAULT_PADDING,
            clip: false,
            class: Theme::default()
        }
    }

    /// Sets the width of the [`Button`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Button`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the [`Padding`] of the [`Button`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed.
    ///
    /// Unless `on_press` is called, the [`Button`] will be disabled.
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(OnPress::Message(on_press));
        self
    }

    pub fn on_press_with_position(
        mut self,
        on_press: impl Fn(ButtonUIRef) -> Message + 'a
    ) -> Self {
        self.on_press = Some(OnPress::MessageWithPosition(Box::new(on_press)));
        self
    }

    /// Sets whether the contents of the [`Button`] should be clipped on
    /// overflow.
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Sets the style of the [`Button`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the [`Id`] of the [`Button`].
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct State {
    is_hovered: bool,
    is_pressed: bool,
    is_focused: bool,
    /// Status the last painted frame was drawn with.
    ///
    /// The runtime only produces a frame when a widget asks for one, so the
    /// button has to compare the status it would paint now against the one the
    /// visible frame carries and request a redraw whenever the two drift apart.
    painted:    Option<Status>
}

/// Resolves the status a button paints itself with for the given cursor.
fn resolve_status(is_pressable: bool, is_mouse_over: bool, is_pressed: bool) -> Status {
    if !is_pressable {
        Status::Disabled
    } else if is_mouse_over {
        if is_pressed {
            Status::Pressed
        } else {
            Status::Hovered
        }
    } else {
        Status::Active
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for PositionButton<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::core::Renderer,
    Theme: Catalog
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

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        Size {
            width:  self.width,
            height: self.height
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits
    ) -> layout::Node {
        layout::padded(limits, self.width, self.height, self.padding, |limits| {
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits)
        })
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation
    ) {
        operation.container(None, layout.bounds());
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            operation
        );
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
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport
        );

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed {
                ..
            }) => {
                if self.on_press.is_some() {
                    let bounds = layout.bounds();

                    if cursor.is_over(bounds) {
                        let state = tree.state.downcast_mut::<State>();

                        state.is_pressed = true;
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted {
                ..
            }) => {
                if let Some(on_press) = self.on_press.as_ref() {
                    let state = tree.state.downcast_mut::<State>();

                    if state.is_pressed {
                        state.is_pressed = false;

                        let bounds = layout.bounds();

                        if cursor.is_over(bounds) {
                            match on_press {
                                OnPress::Message(message) => {
                                    shell.publish(message.clone());
                                }
                                OnPress::MessageWithPosition(on_press) => {
                                    let ui_data = ButtonUIRef {
                                        position: Point::new(
                                            layout.bounds().width / 2. + layout.position().x,
                                            layout.bounds().height / 2. + layout.position().y
                                        ),
                                        viewport: (viewport.width, viewport.height)
                                    };
                                    shell.publish(on_press(ui_data));
                                }
                            }
                        }
                    }
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key, ..
            }) => {
                if let Some(on_press) = self.on_press.as_ref() {
                    let state = tree.state.downcast_mut::<State>();
                    if state.is_focused
                        && matches!(key, keyboard::Key::Named(keyboard::key::Named::Enter))
                    {
                        state.is_pressed = true;
                        match on_press {
                            OnPress::Message(message) => {
                                shell.publish(message.clone());
                            }
                            OnPress::MessageWithPosition(on_press) => {
                                let ui_data = ButtonUIRef {
                                    position: Point::new(
                                        layout.bounds().width / 2. + layout.position().x,
                                        layout.bounds().height / 2. + layout.position().y
                                    ),
                                    viewport: (viewport.width, viewport.height)
                                };
                                shell.publish(on_press(ui_data));
                            }
                        }
                    }
                }
            }
            Event::Touch(touch::Event::FingerLost {
                ..
            })
            | Event::Mouse(mouse::Event::CursorLeft) => {
                let state = tree.state.downcast_mut::<State>();
                state.is_hovered = false;
                state.is_pressed = false;
            }
            _ => {}
        }

        let state = tree.state.downcast_mut::<State>();
        state.is_hovered = cursor.is_over(layout.bounds());

        let current_status =
            resolve_status(self.on_press.is_some(), state.is_hovered, state.is_pressed);

        if matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
            state.painted = Some(current_status);
        } else if state
            .painted
            .is_some_and(|painted| painted != current_status)
        {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle
    ) {
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();
        let state = tree.state.downcast_ref::<State>();

        let status = resolve_status(
            self.on_press.is_some(),
            cursor.is_over(bounds),
            state.is_pressed
        );

        let style = theme.style(&self.class, status);

        if style.background.is_some() || style.border.width > 0.0 || style.shadow.color.a > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: style.border,
                    shadow: style.shadow,
                    snap: false
                },
                style
                    .background
                    .unwrap_or(Background::Color(Color::TRANSPARENT))
            );
        }

        let viewport = if self.clip {
            bounds.intersection(viewport).unwrap_or(*viewport)
        } else {
            *viewport
        };

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color:   style.text_color,
                icon_color:   style.icon_color.unwrap_or(renderer_style.icon_color),
                scale_factor: renderer_style.scale_factor
            },
            content_layout,
            cursor,
            &viewport
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer
    ) -> mouse::Interaction {
        let is_mouse_over = cursor.is_over(layout.bounds());

        if is_mouse_over && self.on_press.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            viewport,
            translation
        )
    }

    fn id(&self) -> Option<Id> {
        Some(self.id.clone())
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
}

impl<'a, Message, Theme, Renderer> From<PositionButton<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: iced::core::Renderer + 'a
{
    fn from(button: PositionButton<'a, Message, Theme, Renderer>) -> Self {
        Self::new(button)
    }
}

pub fn position_button<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>
) -> PositionButton<'a, Message, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: iced::core::Renderer
{
    PositionButton::new(content)
}

/// The default [`Padding`] of a [`Button`].
pub(crate) const DEFAULT_PADDING: Padding = Padding {
    top:    5.0,
    bottom: 5.0,
    right:  10.0,
    left:   10.0
};

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use iced::{
        Theme,
        core::{clipboard, layout::Limits, window::RedrawRequest},
        widget::Space
    };

    use super::*;

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
        let mut tree = Tree::new(&Element::<(), Theme, TestRenderer>::new(button(pressable)));
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

    #[test]
    fn resolves_the_status_the_style_is_picked_from() {
        assert_eq!(resolve_status(false, true, false), Status::Disabled);
        assert_eq!(resolve_status(true, false, false), Status::Active);
        assert_eq!(resolve_status(true, true, false), Status::Hovered);
        assert_eq!(resolve_status(true, true, true), Status::Pressed);
    }
}
