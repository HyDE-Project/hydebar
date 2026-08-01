//! What the module knows about the desktop, and the readers that keep it
//! honest.
//!
//! Everything here either re-reads the state `HyDE` publishes or restates
//! what has already been read — the derived indices, the swatches, the
//! catalogue. Nothing here asks the desktop to change; the surfaces that do
//! live beside this file.

use std::collections::HashMap;

use hydebar_proto::{
    hyde_dirs::HydeDirs,
    hyde_state::{self, HydeState},
    theme_source::{ThemeSwatch, theme_swatch}
};
use iced::Task;

use super::{Message, Spinner, Themes, gallery, view};

/// Reads the swatch of every named theme from the `HyDE` install on disk.
fn read_swatches(themes: &[String]) -> HashMap<String, ThemeSwatch> {
    let Some(dirs) = HydeDirs::from_env() else {
        return HashMap::new();
    };

    themes
        .iter()
        .filter_map(|name| theme_swatch(&dirs, name).map(|swatch| (name.clone(), swatch)))
        .collect()
}

impl Themes {
    /// Starts reading the swatch of every installed theme, off this thread.
    ///
    /// Reading one swatch hashes that theme's current wallpaper, and a dozen
    /// themes make that a moment of real work; done inline it would land in
    /// the middle of the menu's opening animation. The colours arrive through
    /// [`Message::SwatchesLoaded`] and the open menu picks them up on its next
    /// frame.
    #[must_use]
    pub fn load_swatches(&self) -> Task<Message> {
        let themes = self.hyde.themes.clone();

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    (read_swatches(&themes), gallery::local_screenshots())
                })
                .await
                .unwrap_or_default()
            },
            |(swatches, screenshots)| Message::SwatchesLoaded(swatches, screenshots)
        )
    }

    /// Desktop state the module draws.
    #[must_use]
    pub const fn hyde(&self) -> &HydeState {
        &self.hyde
    }

    /// Theme a switch is running for, while one is.
    #[must_use]
    pub fn switching(&self) -> Option<&str> {
        self.switching.as_deref()
    }

    /// Frame the indicator of a running switch is on.
    ///
    /// Read while drawing the bar entry, the module menu and the settings
    /// window alike, so every mark of one wait moves together rather than each
    /// surface keeping a clock of its own.
    #[must_use]
    pub const fn spinner(&self) -> Spinner {
        self.spinner
    }

    /// Whether the bar is waiting on a switch it asked for.
    ///
    /// The application asks for the tick that moves the indicator on only while
    /// this holds.
    #[must_use]
    pub const fn is_waiting(&self) -> bool {
        if self.installing.is_some() || self.updating.is_some() || self.removing.is_some() {
            return true;
        }

        self.switching.is_some()
    }

    /// Re-reads the desktop state `HyDE` publishes.
    ///
    /// Called whenever the bar reloads because a `HyDE` file changed, so a
    /// switch made from a keybinding — or one made here and finished since
    /// — reaches the module without its menu having to be closed and opened
    /// again.
    pub fn refresh(&mut self) {
        self.hyde = hyde_state::load();
        self.reindex();
    }

    /// Restates the offered names and the catalogue index.
    ///
    /// Called when the installed set or the catalogue moves — the only two
    /// inputs the derivations read.
    pub(super) fn reindex(&mut self) {
        let installed: std::collections::HashSet<String> = self
            .hyde
            .themes
            .iter()
            .map(|name| view::canonical(name))
            .collect();

        self.offered = self
            .catalogue
            .iter()
            .filter(|entry| !installed.contains(&view::canonical(&entry.name)))
            .map(|entry| entry.name.clone())
            .collect();

        self.catalogue_index = self
            .catalogue
            .iter()
            .enumerate()
            .map(|(index, entry)| (view::canonical(&entry.name), index))
            .collect();
    }

    /// Starts the catalogue reader, for the gallery section of the menu.
    ///
    /// A catalogue already in hand is kept: the gallery changes on the scale
    /// of weeks, and re-reading it on every menu open would probe the
    /// capability binary and re-parse the index each time. The next bar start
    /// reads it fresh.
    #[must_use]
    pub fn load_catalogue(&self) -> Task<Message> {
        if !self.catalogue.is_empty() {
            return Task::none();
        }

        Task::perform(
            async { (gallery::load().await, gallery::local_author().await) },
            |(catalogue, author)| Message::CatalogueLoaded(catalogue, author)
        )
    }
}
