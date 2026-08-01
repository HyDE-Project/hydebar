//! The gallery install: handing a theme to the desktop's importer, and
//! switching to it once it has landed.

use hydebar_proto::config::Config;
use iced::Task;
use log::info;

use super::{Message, Themes, gallery};
use crate::{services::hyprland_notify::report, utils::hyde_shell};

impl Themes {
    /// Hands a gallery install to the desktop's own importer.
    ///
    /// One at a time, and never beside a running switch: both write the same
    /// theme directories, and two writers racing over them is how a desktop
    /// ends up half one theme and half another.
    pub(super) fn install(&mut self, theme: String, config: &Config) -> Task<Message> {
        if self.removing.is_some() {
            report(config, "a removal is running, installs must wait");
            return Task::none();
        }

        if let Some(pending) = self.switching.as_deref() {
            info!("ignoring the install of `{theme}`: `{pending}` is still being applied");
            return Task::none();
        }

        if let Some(pending) = self.installing.as_deref() {
            info!("ignoring the install of `{theme}`: `{pending}` is still being installed");
            return Task::none();
        }

        let Some(entry) = self.catalogue.iter().find(|entry| entry.name == theme) else {
            report(
                config,
                &format!("the gallery lists no theme named `{theme}`")
            );
            return Task::none();
        };

        info!("installing the HyDE theme `{theme}` from `{}`", entry.link);
        self.installing = Some(theme.clone());

        let command = gallery::import_command(&entry.name, &entry.link);

        Task::perform(hyde_shell::run(command), move |failure| {
            Message::Installed {
                theme,
                failure
            }
        })
    }

    /// Records what the desktop made of the install that just ended, and
    /// switches to the theme once it has landed.
    pub(super) fn installed(
        &mut self,
        theme: String,
        failure: Option<String>,
        config: &Config
    ) -> Task<Message> {
        self.installing = None;

        if let Some(failure) = failure {
            report(
                config,
                &format!("installing the HyDE theme `{theme}` failed: {failure}")
            );

            return Task::none();
        }

        info!("the HyDE theme `{theme}` is installed, switching to it");
        self.refresh();

        Task::batch([self.load_swatches(), self.switch(theme, config)])
    }
}
