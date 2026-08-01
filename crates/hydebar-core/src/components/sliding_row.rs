//! A row whose children glide to their new places instead of jumping.
//!
//! When the bar's arrangement changes, every block is given a key and the
//! row remembers where each key last stood. For as long as the caller's
//! transition is travelling, a surviving block is drawn — and hit — at a
//! position interpolated between its old seat and its new one, so a layout
//! switch reads as furniture sliding across the shelf rather than as a cut.

use iced::{
    Length, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget,
        event::Event,
        layout, mouse, renderer,
        widget::{Operation, Tree, tree}
    }
};

/// Where each key stood when the row last rested.
#[derive(Debug, Clone, Default)]
struct State {
    /// Positions the settled row assigned, by child key.
    settled: std::collections::HashMap<u64, f32>,
    /// Positions the current slide departs from, by child key.
    from:    std::collections::HashMap<u64, f32>
}

/// A horizontal row of keyed children with slide-to-place drawing.
pub struct SlidingRow<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    children: Vec<iced_core::Element<'a, Message, Theme, Renderer>>,
    keys:     Vec<u64>,
    spacing:  f32,
    /// How far the slide has travelled, one meaning at rest.
    progress: f32
}

impl<Message, Theme, Renderer> std::fmt::Debug for SlidingRow<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlidingRow")
            .field("keys", &self.keys)
            .field("spacing", &self.spacing)
            .field("progress", &self.progress)
            .finish_non_exhaustive()
    }
}

impl<'a, Message, Theme, Renderer> SlidingRow<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    /// Builds an empty row travelling at `progress`.
    #[must_use]
    pub const fn new(spacing: f32, progress: f32) -> Self {
        Self {
            children: Vec::new(),
            keys: Vec::new(),
            spacing,
            progress
        }
    }

    /// Appends a child under its stable key.
    #[must_use]
    pub fn push(
        mut self,
        key: u64,
        child: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>
    ) -> Self {
        self.children.push(child.into());
        self.keys.push(key);
        self
    }

    /// The drawing offset of the child at `index`, given its resting `x`.
    fn offset(&self, state: &State, index: usize, natural_x: f32) -> f32 {
        if self.progress >= 1.0 {
            return 0.0;
        }

        let key = self.keys[index];

        state
            .from
            .get(&key)
            .map_or(0.0, |from| (from - natural_x) * (1.0 - self.progress))
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for SlidingRow<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits
    ) -> layout::Node {
        let loose = limits.loose();
        let mut nodes = Vec::with_capacity(self.children.len());
        let mut x = 0.0f32;
        let mut height = 0.0f32;

        for (child, sub_tree) in self.children.iter_mut().zip(tree.children.iter_mut()) {
            let node = child.as_widget_mut().layout(sub_tree, renderer, &loose);
            height = height.max(node.size().height);
            nodes.push((x, node));
            x += nodes.last().map_or(0.0, |(_, node)| node.size().width) + self.spacing;
        }

        let width = (x - self.spacing).max(0.0);

        let placed: Vec<layout::Node> = nodes
            .into_iter()
            .map(|(x, node)| {
                let dy = (height - node.size().height) / 2.0;
                node.move_to(iced::Point::new(x, dy))
            })
            .collect();

        let state = tree.state.downcast_mut::<State>();

        if self.progress >= 1.0 {
            state.settled = self
                .keys
                .iter()
                .zip(placed.iter())
                .map(|(key, node)| (*key, node.bounds().x))
                .collect();
            state.from.clear();
        } else if state.from.is_empty() {
            state.from = state.settled.clone();
        }

        layout::Node::with_children(Size::new(width, height), placed)
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
        let origin = layout.position();
        let offsets: Vec<f32> = {
            let state = tree.state.downcast_ref::<State>();

            layout
                .children()
                .enumerate()
                .map(|(index, child_layout)| {
                    self.offset(state, index, child_layout.bounds().x - origin.x)
                })
                .collect()
        };

        for (index, ((child, sub_tree), child_layout)) in self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
            .enumerate()
        {
            let shifted = match cursor {
                mouse::Cursor::Available(point) => {
                    mouse::Cursor::Available(point - Vector::new(offsets[index], 0.0))
                }
                other => other
            };

            child.as_widget_mut().update(
                sub_tree,
                event,
                child_layout,
                shifted,
                renderer,
                clipboard,
                shell,
                viewport
            );
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
        let origin = layout.position();
        let state = tree.state.downcast_ref::<State>();

        for (index, ((child, sub_tree), child_layout)) in self
            .children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .enumerate()
        {
            let offset = self.offset(state, index, child_layout.bounds().x - origin.x);

            if offset.abs() < f32::EPSILON {
                child.as_widget().draw(
                    sub_tree,
                    renderer,
                    theme,
                    style,
                    child_layout,
                    cursor,
                    viewport
                );
            } else {
                renderer.with_translation(Vector::new(offset, 0.0), |renderer| {
                    child.as_widget().draw(
                        sub_tree,
                        renderer,
                        theme,
                        style,
                        child_layout,
                        cursor,
                        viewport
                    );
                });
            }
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
        let origin = layout.position();
        let state = tree.state.downcast_ref::<State>();

        self.children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .enumerate()
            .map(|(index, ((child, sub_tree), child_layout))| {
                let offset = self.offset(state, index, child_layout.bounds().x - origin.x);
                let shifted = match cursor {
                    mouse::Cursor::Available(point) => {
                        mouse::Cursor::Available(point - Vector::new(offset, 0.0))
                    }
                    other => other
                };

                child.as_widget().mouse_interaction(
                    sub_tree,
                    child_layout,
                    shifted,
                    viewport,
                    renderer
                )
            })
            .max()
            .unwrap_or_default()
    }
}

impl<'a, Message, Theme, Renderer> From<SlidingRow<'a, Message, Theme, Renderer>>
    for iced_core::Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced_core::Renderer + 'a
{
    fn from(row: SlidingRow<'a, Message, Theme, Renderer>) -> Self {
        Self::new(row)
    }
}
