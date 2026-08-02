//! Message folding and command dispatch for the wallpaper module.

use hydebar_proto::config::Config;
use iced::Task;
use log::error;

use super::{Message, Wallpaper, listing};
use crate::{services::hyprland_notify::report, utils::hyde_shell};

impl Wallpaper {
    /// Starts reading the theme's wallpapers, off this thread.
    ///
    /// Thumbnails already decoded ride along and are reused by path, so a
    /// reopened picker decodes nothing and a theme switch decodes only the
    /// pictures it brought.
    #[must_use]
    pub fn load_entries(&mut self) -> Task<Message> {
        self.loading = true;

        let known: std::collections::HashMap<String, iced::widget::image::Handle> = self
            .entries
            .iter()
            .map(|entry| (entry.path.clone(), entry.thumbnail.clone()))
            .collect();

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || listing::list_wallpapers(&known))
                    .await
                    .unwrap_or_default()
            },
            Message::Listed
        )
    }

    /// Applies a press made on the module.
    pub fn update(&mut self, message: Message, config: &Config) -> Task<Message> {
        match message {
            Message::Next => {
                return Task::perform(hyde_shell::run(hyde_shell::next_wallpaper()), |failure| {
                    Message::Changed {
                        failure
                    }
                });
            }
            Message::Previous => {
                return Task::perform(
                    hyde_shell::run(hyde_shell::previous_wallpaper()),
                    |failure| Message::Changed {
                        failure
                    }
                );
            }
            Message::Changed {
                failure
            } => {
                if let Some(reason) = failure {
                    error!("the wallpaper could not be changed: {reason}");
                    report(config, "the desktop refused to change the wallpaper");
                }
            }
            Message::Listed(entries) => {
                self.entries = entries;
                self.loading = false;
            }
            Message::Tick => {
                self.spinner.advance();
            }
            Message::Pick(path) => {
                let command = format!("hydectl wallpaper set '{}'", path.replace('\'', "'\\''"));

                return Task::perform(hyde_shell::run(command), |failure| Message::Changed {
                    failure
                });
            }
        }

        Task::none()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_two_directions_ask_the_desktop_for_different_things() {
        assert_ne!(
            hyde_shell::next_wallpaper(),
            hyde_shell::previous_wallpaper()
        );
    }

    #[test]
    fn a_refused_change_is_reported_rather_than_swallowed() {
        let mut wallpaper = Wallpaper::default();

        let task = wallpaper.update(
            Message::Changed {
                failure: Some("no".to_owned())
            },
            &Config::default()
        );

        let _task: Task<Message> = task;
    }
}
