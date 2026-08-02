//! The removal of a condemned theme, and the record of how it ended.

use hydebar_proto::config::Config;
use iced::Task;
use log::info;

use super::{Message, Themes};
use crate::services::hyprland_notify::report;

impl Themes {
    /// Removes a condemned theme's directory, once everything checks out.
    ///
    /// Only a theme the removal was armed for goes, never the one the desktop
    /// is on, never during a switch or an install, and strictly the directory
    /// the installed list names — nothing about the path comes from outside.
    pub(super) fn remove(&mut self, theme: String, config: &Config) -> Task<Message> {
        if self.removing.is_some() {
            report(config, "a removal is already running, one at a time");
            return Task::none();
        }

        if self.switching.is_some() {
            report(config, "a theme switch is running, removal must wait");
            return Task::none();
        }

        if self.installing.is_some() {
            report(config, "a theme install is running, removal must wait");
            return Task::none();
        }

        if self.updating.is_some() {
            report(config, "a theme update is fetching, removal must wait");
            return Task::none();
        }

        if self.hyde.theme.as_deref() == Some(theme.as_str()) {
            report(
                config,
                "the theme in force cannot be removed, switch away first"
            );
            return Task::none();
        }

        if !self.hyde.themes.contains(&theme) {
            report(config, "this theme is not installed");
            return Task::none();
        }

        let Some(directory) = dirs::config_dir().map(|dir| dir.join("hyde/themes").join(&theme))
        else {
            report(config, "the theme directory cannot be located");
            return Task::none();
        };

        info!(
            "removing the HyDE theme `{theme}` at {}",
            directory.display()
        );

        self.removing = Some(theme.clone());

        Task::perform(
            async move {
                tokio::fs::remove_dir_all(directory)
                    .await
                    .err()
                    .map(|error| error.to_string())
            },
            move |failure| Message::Removed {
                theme,
                failure
            }
        )
    }

    /// Records what the desktop made of the removal that just ended.
    pub(super) fn removed(
        &mut self,
        theme: &str,
        failure: Option<&str>,
        config: &Config
    ) -> Task<Message> {
        self.removing = None;

        match failure {
            Some(failure) => report(
                config,
                &format!("removing the HyDE theme `{theme}` failed: {failure}")
            ),
            None => report(config, &format!("the HyDE theme `{theme}` is removed"))
        }

        self.refresh();

        self.load_swatches()
    }
}
