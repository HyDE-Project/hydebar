//! Building the row: children under stable keys, and the offset each one
//! is drawn at while a slide travels.

use super::state::State;

/// A horizontal row of keyed children with slide-to-place drawing.
pub struct SlidingRow<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced_core::Renderer
{
    pub(super) children: Vec<iced_core::Element<'a, Message, Theme, Renderer>>,
    pub(super) keys:     Vec<u64>,
    pub(super) spacing:  f32,
    /// How far the slide has travelled, one meaning at rest.
    pub(super) progress: f32
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
    pub(super) fn offset(&self, state: &State, index: usize, natural_x: f32) -> f32 {
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
