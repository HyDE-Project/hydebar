//! The one place a choice made in the module is turned into work.
//!
//! Every [`Message`] lands here, and each is handed to the surface that owns
//! it — the switch, the install, the removal, the update fetch — so the
//! ordering rules between them can be read in one match.

use hydebar_proto::config::Config;
use iced::Task;
use log::{error, info};

use super::{Message, Themes};
use crate::{services::hyprland_notify::report, utils::hyde_shell};

impl Themes {
    /// Applies a choice made in the module.
    ///
    /// Nothing about the desktop is assumed: the module reports that a switch
    /// is running, and what the desktop settled on is read back off disk once
    /// it has, so a switch that never happened is never drawn as if it had.
    pub fn update(&mut self, message: Message, config: &Config) -> Task<Message> {
        match message {
            Message::NextTheme => return Self::step("hydectl theme next"),
            Message::PreviousTheme => return Self::step("hydectl theme prev"),
            Message::Stepped {
                failure
            } => {
                if let Some(reason) = failure {
                    error!("the theme step failed: {reason}");
                    report(config, "the desktop refused to step the theme");
                }

                self.refresh();

                return self.load_swatches();
            }
            Message::Switch(theme) => {
                info!("theme chip pressed: `{theme}`");

                return self.switch(theme, config);
            }
            Message::Switched {
                theme,
                failure
            } => {
                self.switched(&theme, failure.as_deref(), config);

                if let Some(next) = self.pending.take() {
                    return Task::batch([self.load_swatches(), self.switch(next, config)]);
                }

                return self.load_swatches();
            }
            Message::Tick => {
                if self.switching.is_some() || self.installing.is_some() || self.updating.is_some()
                {
                    self.spinner.advance();
                }
            }
            Message::SwatchesLoaded(swatches, screenshots) => {
                self.swatches = swatches;
                self.screenshots = screenshots;
            }
            Message::CatalogueLoaded(catalogue, author) => {
                self.catalogue = catalogue;
                self.reindex();
                self.author = author;
                return self.auto_update();
            }
            Message::Update(scope) => return self.fetch_updates(scope),
            Message::ToggleLayout => self.list_layout = !self.list_layout,
            Message::Updated {
                failure
            } => {
                self.updating = None;

                if let Some(failure) = failure {
                    report(config, &format!("updating HyDE themes failed: {failure}"));
                }

                self.refresh();

                return self.load_swatches();
            }
            Message::Install(theme) => return self.install(theme, config),
            Message::Remove(theme) => return self.remove(theme, config),
            Message::Removed {
                theme,
                failure
            } => {
                return self.removed(&theme, failure.as_deref(), config);
            }
            Message::Installed {
                theme,
                failure
            } => {
                return self.installed(theme, failure, config);
            }
        }

        Task::none()
    }

    /// Asks the desktop to step to a neighbouring theme in its own order.
    fn step(command: &str) -> Task<Message> {
        Task::perform(hyde_shell::run(command.to_owned()), |failure| {
            Message::Stepped {
                failure
            }
        })
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn waiting_on(theme: &str) -> Themes {
        Themes {
            switching: Some(theme.to_owned()),
            ..Themes::default()
        }
    }

    fn tick(themes: &mut Themes) {
        let _ = themes.update(Message::Tick, &Config::default());
    }

    #[test]
    fn a_finished_switch_releases_the_module_for_the_next_one() {
        let mut themes = waiting_on("Tokyo Night");

        let _ = themes.update(
            Message::Switched {
                theme:   "Tokyo Night".to_owned(),
                failure: None
            },
            &Config::default()
        );

        assert_eq!(themes.switching(), None);
        assert!(!themes.is_waiting());
    }

    #[test]
    fn a_module_that_is_not_switching_reports_nothing_pending() {
        assert_eq!(Themes::default().switching(), None);
        assert!(!Themes::default().is_waiting());
    }

    #[test]
    fn a_tick_moves_the_indicator_of_a_running_switch_on() {
        let mut themes = waiting_on("Tokyo Night");
        let before = themes.spinner();

        tick(&mut themes);

        assert_ne!(themes.spinner(), before);
    }

    /// The tick is only asked for while a switch runs, but a tick already in
    /// flight when one ends must not leave the indicator on a frame nobody
    /// draws.
    #[test]
    fn a_tick_arriving_after_the_switch_ended_moves_nothing() {
        let mut themes = Themes::default();
        let before = themes.spinner();

        tick(&mut themes);

        assert_eq!(themes.spinner(), before);
    }

    #[test]
    fn a_failed_switch_takes_the_indicator_off_the_bar() {
        let mut themes = waiting_on("Tokyo Night");

        let _ = themes.update(
            Message::Switched {
                theme:   "Tokyo Night".to_owned(),
                failure: Some("the script died".to_owned())
            },
            &Config::default()
        );

        assert!(!themes.is_waiting());
    }
}
