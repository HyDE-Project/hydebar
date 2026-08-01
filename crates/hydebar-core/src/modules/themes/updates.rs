//! The update fetch: the one the button runs, and the quiet daily one.

use iced::Task;
use log::info;

use super::{Message, Themes};
use crate::utils::hyde_shell;

impl Themes {
    /// Fetches theme updates through the desktop's own importer.
    ///
    /// One writer at a time over the theme directories: a fetch never starts
    /// beside a switch, an install, or another fetch.
    pub(super) fn fetch_updates(&mut self, scope: Option<String>) -> Task<Message> {
        if self.switching.is_some()
            || self.installing.is_some()
            || self.updating.is_some()
            || self.removing.is_some()
        {
            return Task::none();
        }

        let command = scope.as_ref().map_or_else(
            || "hyde-shell theme.import --fetch all".to_owned(),
            |theme| {
                format!(
                    "hyde-shell theme.import --fetch '{}'",
                    theme.replace('\'', "'\\''")
                )
            }
        );

        info!("fetching HyDE theme updates: {command}");
        self.updating = Some(scope);

        Task::perform(hyde_shell::run(command), |failure| Message::Updated {
            failure
        })
    }

    /// Fetches all updates quietly, at most once a day.
    ///
    /// The professional half of the update button: opening the window checks
    /// a stamp beside the catalogue cache, and a stale stamp starts the same
    /// fetch the button runs — silently, with the same one-writer guards.
    pub(super) fn auto_update(&mut self) -> Task<Message> {
        const STAMP_LIFE: std::time::Duration = std::time::Duration::from_hours(24);

        let Some(stamp) = dirs::cache_dir().map(|dir| dir.join("hydebar/theme-update-stamp"))
        else {
            return Task::none();
        };

        let fresh = std::fs::metadata(&stamp)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < STAMP_LIFE);

        if fresh {
            return Task::none();
        }

        let fetch = self.fetch_updates(None);

        if self.updating.is_none() {
            return Task::none();
        }

        if let Some(dir) = stamp.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&stamp, b"");

        fetch
    }
}
