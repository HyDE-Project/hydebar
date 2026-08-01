//! How the strip answers the widget tree: bookkeeping stays here, while
//! seating, drawing and event delivery are delegated to their own files.

use iced::{
    Length, Rectangle, Size,
    core::{
        Clipboard, Layout, Shell, Widget,
        event::Event,
        layout, mouse, renderer,
        widget::{Operation, Tree, tree}
    }
};

use super::{builder::Archipelago, draw, events, layout as seating};

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Archipelago<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits
    ) -> layout::Node {
        seating::layout(self, tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation
    ) {
        for ((child, sub_tree), child_layout) in self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            child
                .as_widget_mut()
                .operate(sub_tree, child_layout, renderer, operation);
        }
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
}

impl<'a, Message, Theme, Renderer> From<Archipelago<'a, Message, Theme, Renderer>>
    for iced_core::Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced_core::Renderer + 'a
{
    fn from(strip: Archipelago<'a, Message, Theme, Renderer>) -> Self {
        Self::new(strip)
    }
}
