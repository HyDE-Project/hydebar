//! What the bar entry and the menu read out of the state.

use iced::{Element, SurfaceId as Id};

use super::{super::view, CheckState, HydeSnapshot, Message, Update, Updates};
use crate::components::icons::IconTheme;

impl Updates {
    /// Hint shown while the pointer rests on the bar entry.
    ///
    /// Nothing is shown where no check can run: the entry itself is absent
    /// from such a bar, so there is nothing to explain.
    #[must_use]
    pub fn tooltip(&self) -> Option<String> {
        let line = match self.state {
            CheckState::Checking => "Updates: checking".to_owned(),
            CheckState::Ready => match self.pending.len() {
                0 => "Updates: none pending".to_owned(),
                pending => format!("Updates: {pending} pending")
            },
            CheckState::Unavailable => return None
        };

        Some(match self.hyde_pending() {
            0 => line,
            behind => format!("{line} · HyDE: {behind} commits behind")
        })
    }

    /// Folds both open lists shut, the state a freshly opened menu shows.
    ///
    /// Leftover narration goes with them: a log that outlived its run was
    /// read in the window that witnessed it, and a window opened anew
    /// should not still be reporting last time's weather. A run still
    /// going keeps its lines.
    pub fn collapse(&mut self) {
        self.is_updates_list_open = false;
        self.is_hyde_list_open = false;

        if !self.applying {
            self.apply_log.clear();
            self.apply_failed = false;
        }

        if !self.hyde_updating {
            self.hyde_log.clear();
            self.hyde_failed = false;
        }
    }

    /// How many upstream commits the `HyDE` clone has not taken yet.
    pub(super) fn hyde_pending(&self) -> usize {
        self.hyde
            .as_ref()
            .map_or(0, |snapshot| snapshot.commits.len())
    }

    /// Advances the dissolve of the count on the bar.
    pub fn tick_fade(&mut self, elapsed: std::time::Duration) -> bool {
        self.shown_count.advance(elapsed)
    }

    /// Whether the count on the bar is still dissolving.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        self.shown_count.is_animating()
    }

    #[must_use]
    pub fn menu_view(&self, id: Id, opacity: f32, icons: &IconTheme) -> Element<'_, Message> {
        view::menu_view(self, id, opacity, icons)
    }

    pub(crate) fn updates(&self) -> &[Update] {
        &self.pending
    }

    pub(crate) const fn is_updates_list_open(&self) -> bool {
        self.is_updates_list_open
    }

    pub(crate) const fn state(&self) -> &CheckState {
        &self.state
    }

    pub(crate) const fn hyde(&self) -> Option<&HydeSnapshot> {
        self.hyde.as_ref()
    }

    pub(crate) const fn is_hyde_list_open(&self) -> bool {
        self.is_hyde_list_open
    }

    pub(crate) const fn is_hyde_updating(&self) -> bool {
        self.hyde_updating
    }

    pub(crate) fn hyde_log(&self) -> &[String] {
        &self.hyde_log
    }

    pub(crate) const fn is_applying(&self) -> bool {
        self.applying
    }

    pub(crate) fn apply_log(&self) -> &[String] {
        &self.apply_log
    }

    /// Branch the `HyDE` clone is measured against.
    pub(crate) fn hyde_branch_name(&self) -> &str {
        self.hyde_branch.as_deref().unwrap_or("master")
    }
}
