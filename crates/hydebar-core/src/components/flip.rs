//! Every block glides to its new seat across the whole panel.
//!
//! Each module is wrapped in an anchor that records its absolute position
//! on every frame into one shared registry. The moment the arrangement
//! changes, the registry's live positions are frozen into a departure map;
//! for as long as the caller's transition travels, every surviving module
//! is drawn — and hit — between its old seat and its new one, wherever on
//! the panel both happen to be. A module changing islands or sections
//! therefore flies there as one piece instead of reappearing.

use std::{cell::RefCell, collections::HashMap};

use iced::{
    Length, Rectangle, Size, Vector,
    core::{
        Clipboard, Layout, Shell, Widget,
        event::Event,
        layout, mouse, renderer,
        widget::{Operation, Tree, tree}
    }
};

/// The shared book of seats: where every key is, and where it came from.
#[derive(Debug, Default)]
pub struct FlipMemo {
    /// Absolute positions as they were drawn on the latest frame.
    live: HashMap<u64, f32>,
    /// Absolute positions the current journey departs from.
    from: HashMap<u64, f32>
}

impl FlipMemo {
    /// Freezes the live seats as the departure points of a new journey.
    ///
    /// Called at the moment a rearrangement is adopted, before the next
    /// frame lays the new arrangement out.
    pub fn depart(&mut self) {
        self.from = self.live.clone();
    }
}

/// Wraps one block so it journeys between its recorded seats.
pub struct FlipAnchor<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    key:      u64,
    /// How far the journey has travelled, one meaning at rest.
    progress: f32,
    memo:     &'a RefCell<FlipMemo>,
    content:  iced_core::Element<'a, Message, Theme, Renderer>
}

impl<'a, Message, Theme, Renderer> FlipAnchor<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    /// Wraps `content` under its stable `key`.
    pub fn new(
        key: u64,
        progress: f32,
        memo: &'a RefCell<FlipMemo>,
        content: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>
    ) -> Self {
        Self {
            key,
            progress,
            memo,
            content: content.into()
        }
    }

    /// The drawing offset for the current frame, given the resting seat.
    fn offset(&self, x: f32) -> f32 {
        if self.progress >= 1.0 {
            return 0.0;
        }

        self.memo
            .borrow()
            .from
            .get(&self.key)
            .map_or(0.0, |from| (from - x) * (1.0 - self.progress))
    }
}

impl<Message, Theme, Renderer> std::fmt::Debug for FlipAnchor<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlipAnchor")
            .field("key", &self.key)
            .field("progress", &self.progress)
            .finish_non_exhaustive()
    }
}

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

        self.memo.borrow_mut().live.insert(self.key, x);

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
