//! The [`Widget`] implementation wiring a [`Centerbox`] into iced.

use iced::{
    Event, Length, Rectangle, Size, Vector,
    advanced::{
        Clipboard, Shell, Widget,
        layout::{self, Layout},
        mouse, overlay, renderer,
        widget::{Operation, Tree}
    }
};

use super::{builder::Centerbox, draw, events, layout as centerbox_layout};

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Centerbox<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.children)
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
        centerbox_layout::layout(self, tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation
    ) {
        events::operate(self, tree, layout, renderer, operation);
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
        events::update(
            self, tree, event, layout, cursor, renderer, clipboard, shell, viewport
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
        events::mouse_interaction(self, tree, layout, cursor, viewport, renderer)
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
        draw::draw(self, tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        events::overlay(self, tree, layout, renderer, viewport, translation)
    }
}
