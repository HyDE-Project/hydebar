//! How the anchor answers the widget tree: everything is delegated to the
//! wrapped block, drawing and hitting shifted by the journey's offset.

use iced::{
    Length, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget,
        event::Event,
        layout, mouse, renderer,
        widget::{Operation, Tree, tree}
    }
};

use super::anchor::FlipAnchor;

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for FlipAnchor<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
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
        self.content.as_widget_mut().layout(tree, renderer, limits)
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
            .operate(tree, layout, renderer, operation);
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
        let offset = self.offset(layout.position().x);
        let shifted = match cursor {
            mouse::Cursor::Available(point) if offset.abs() > f32::EPSILON => {
                mouse::Cursor::Available(point - Vector::new(offset, 0.0))
            }
            other => other
        };

        self.content.as_widget_mut().update(
            tree, event, layout, shifted, renderer, clipboard, shell, viewport
        );
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
        let x = layout.position().x;

        self.memo.borrow_mut().record(self.key, x);

        let offset = self.offset(x);

        if offset.abs() < f32::EPSILON {
            self.content
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        } else {
            renderer.with_translation(Vector::new(offset, 0.0), |renderer| {
                self.content
                    .as_widget()
                    .draw(tree, renderer, theme, style, layout, cursor, viewport);
            });
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer
    ) -> mouse::Interaction {
        let offset = self.offset(layout.position().x);
        let shifted = match cursor {
            mouse::Cursor::Available(point) if offset.abs() > f32::EPSILON => {
                mouse::Cursor::Available(point - Vector::new(offset, 0.0))
            }
            other => other
        };

        self.content
            .as_widget()
            .mouse_interaction(tree, layout, shifted, viewport, renderer)
    }
}

impl<'a, Message, Theme, Renderer> From<FlipAnchor<'a, Message, Theme, Renderer>>
    for iced_core::Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced_core::Renderer + 'a
{
    fn from(anchor: FlipAnchor<'a, Message, Theme, Renderer>) -> Self {
        Self::new(anchor)
    }
}
