//! Islands drawn under wherever the icons actually are.
//!
//! The pill behind a group of modules is not a box the modules live in —
//! it is painted every frame around the places the modules currently
//! stand. At rest that reproduces the configured islands exactly. While a
//! rearrangement travels, every module glides from its old seat to its
//! new one carrying a pill of its own, and pills of modules that draw
//! near each other fuse into one island, then part again as they pass —
//! no icon is ever bare, and islands form under the arriving furniture.

use std::{cell::RefCell, collections::HashMap};

use iced::{
    Border, Length, Rectangle, Shadow, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget,
        event::Event,
        layout, mouse, renderer,
        widget::{Operation, Tree, tree}
    }
};

/// The paint one pill is drawn with.
#[derive(Debug, Clone, Copy)]
pub struct PillPaint {
    /// Fill of the pill.
    pub background: iced::Color,
    /// Border the pill carries, radius included.
    pub border:     Border,
    /// Shadow under the pill.
    pub shadow:     Shadow
}

/// The shared book of seats, reused from the flip machinery.
pub use super::flip::FlipMemo;

/// How one pill asks to be painted for the theme in force.
type PaintFn<'a, Theme> = Box<dyn Fn(&Theme) -> Option<PillPaint> + 'a>;

/// A strip of keyed modules whose island pills follow them around.
pub struct Archipelago<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    children:   Vec<iced_core::Element<'a, Message, Theme, Renderer>>,
    keys:       Vec<u64>,
    /// Which configured island each child belongs to, in order.
    islands:    Vec<usize>,
    island_gap: f32,
    /// Horizontal room a pill keeps around its content.
    pad_x:      f32,
    /// How far the journey has travelled, one meaning at rest.
    progress:   f32,
    memo:       &'a RefCell<FlipMemo>,
    paint:      PaintFn<'a, Theme>
}

impl<'a, Message, Theme, Renderer> Archipelago<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    /// Builds an empty strip travelling at `progress`.
    pub fn new(
        island_gap: f32,
        pad_x: f32,
        progress: f32,
        memo: &'a RefCell<FlipMemo>,
        paint: impl Fn(&Theme) -> Option<PillPaint> + 'a
    ) -> Self {
        Self {
            children: Vec::new(),
            keys: Vec::new(),
            islands: Vec::new(),
            island_gap,
            pad_x,
            progress,
            memo,
            paint: Box::new(paint)
        }
    }

    /// Appends a module under its stable key, seated on `island`.
    #[must_use]
    pub fn push(
        mut self,
        key: u64,
        island: usize,
        child: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>
    ) -> Self {
        self.children.push(child.into());
        self.keys.push(key);
        self.islands.push(island);
        self
    }

    /// Whether the strip holds nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// The drawing offset of the child at `index`, given its absolute `x`.
    fn offset(&self, from: &HashMap<u64, f32>, index: usize, x: f32) -> f32 {
        if self.progress >= 1.0 {
            return 0.0;
        }

        from.get(&self.keys[index])
            .map_or(0.0, |from| (from - x) * (1.0 - self.progress))
    }
}

impl<Message, Theme, Renderer> std::fmt::Debug for Archipelago<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Archipelago")
            .field("keys", &self.keys)
            .field("islands", &self.islands)
            .field("progress", &self.progress)
            .finish_non_exhaustive()
    }
}

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
        let loose = limits.loose();
        let height = limits.max().height;
        let mut x = 0.0f32;
        let mut placed = Vec::with_capacity(self.children.len());

        let count = self.children.len();
        let islands = self.islands.clone();
        let island_gap = self.island_gap;
        let pad_x = self.pad_x;

        for (index, (child, sub_tree)) in self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
        {
            let opens = index == 0 || islands[index] != islands[index - 1];
            let closes = index + 1 == count || islands[index + 1] != islands[index];

            if opens {
                if index > 0 {
                    x += island_gap;
                }
                x += pad_x;
            }

            let node = child.as_widget_mut().layout(sub_tree, renderer, &loose);
            let dy = ((height - node.size().height) / 2.0).max(0.0);
            let width = node.size().width;

            placed.push(node.move_to(iced::Point::new(x, dy)));
            x += width;

            if closes {
                x += pad_x;
            }
        }

        layout::Node::with_children(Size::new(x, height), placed)
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
        let offsets: Vec<f32> = {
            let memo = self.memo.borrow();

            layout
                .children()
                .enumerate()
                .map(|(index, child_layout)| {
                    self.offset(memo.from_map(), index, child_layout.bounds().x)
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
                mouse::Cursor::Available(point) if offsets[index].abs() > f32::EPSILON => {
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
        let bounds = layout.bounds();

        let (offsets, spans) = {
            let mut memo = self.memo.borrow_mut();
            let mut offsets = Vec::with_capacity(self.children.len());
            let mut spans: Vec<(f32, f32)> = Vec::with_capacity(self.children.len());

            for (index, child_layout) in layout.children().enumerate() {
                let child_bounds = child_layout.bounds();

                let offset = self.offset(memo.from_map(), index, child_bounds.x);
                let current = child_bounds.x + offset;

                memo.record(self.keys[index], current);

                offsets.push(offset);
                spans.push((
                    current - self.pad_x,
                    current + child_bounds.width + self.pad_x
                ));
            }

            (offsets, spans)
        };

        if let Some(paint) = (self.paint)(theme) {
            let mut clusters: Vec<(f32, f32)> = Vec::new();
            let mut sorted = spans;
            sorted.sort_by(|a, b| a.0.total_cmp(&b.0));

            let fuse = self.island_gap * 0.5;

            for (start, end) in sorted {
                match clusters.last_mut() {
                    Some((_, tail)) if start - *tail <= fuse => {
                        *tail = tail.max(end);
                    }
                    _ => clusters.push((start, end))
                }
            }

            for (start, end) in clusters {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x:      start,
                            y:      bounds.y,
                            width:  end - start,
                            height: bounds.height
                        },
                        border: paint.border,
                        shadow: paint.shadow,
                        ..renderer::Quad::default()
                    },
                    paint.background
                );
            }
        }

        for (index, ((child, sub_tree), child_layout)) in self
            .children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .enumerate()
        {
            if offsets[index].abs() < f32::EPSILON {
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
                renderer.with_translation(Vector::new(offsets[index], 0.0), |renderer| {
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
        let offsets: Vec<f32> = {
            let memo = self.memo.borrow();

            layout
                .children()
                .enumerate()
                .map(|(index, child_layout)| {
                    self.offset(memo.from_map(), index, child_layout.bounds().x)
                })
                .collect()
        };

        self.children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .enumerate()
            .map(|(index, ((child, sub_tree), child_layout))| {
                let _ = child_layout;
                let shifted = match cursor {
                    mouse::Cursor::Available(point) if offsets[index].abs() > f32::EPSILON => {
                        mouse::Cursor::Available(point - Vector::new(offsets[index], 0.0))
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
