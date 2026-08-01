//! Building the strip: pill paint, keyed modules and their seats.

use std::{cell::RefCell, collections::HashMap};

use iced::{Border, Shadow};

use super::FlipMemo;

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

/// How one pill asks to be painted for the theme in force.
type PaintFn<'a, Theme> = Box<dyn Fn(&Theme) -> Option<PillPaint> + 'a>;

/// A strip of keyed modules whose island pills follow them around.
pub struct Archipelago<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    pub(super) children:   Vec<iced_core::Element<'a, Message, Theme, Renderer>>,
    pub(super) keys:       Vec<u64>,
    /// Which configured island each child belongs to, in order.
    pub(super) islands:    Vec<usize>,
    /// How far each child's entrance wave has arrived, one meaning fully.
    pub(super) arrivals:   Vec<f32>,
    pub(super) island_gap: f32,
    /// Horizontal room a pill keeps around its content.
    pub(super) pad_x:      f32,
    /// How far the journey has travelled, one meaning at rest.
    pub(super) progress:   f32,
    pub(super) memo:       &'a RefCell<FlipMemo>,
    pub(super) paint:      PaintFn<'a, Theme>
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
            arrivals: Vec::new(),
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
        arrival: f32,
        child: impl Into<iced_core::Element<'a, Message, Theme, Renderer>>
    ) -> Self {
        self.children.push(child.into());
        self.keys.push(key);
        self.islands.push(island);
        self.arrivals.push(arrival.clamp(0.0, 1.0));
        self
    }

    /// Whether the strip holds nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// The drawing offset of the child at `index`, given its absolute `x`.
    pub(super) fn offset(&self, from: &HashMap<u64, f32>, index: usize, x: f32) -> f32 {
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
