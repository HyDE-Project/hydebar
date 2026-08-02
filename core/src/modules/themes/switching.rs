//! The switch itself: what a press on a chip is allowed to start, the wait
//! it opens, and the record of how the desktop answered.

use hydebar_proto::config::Config;
use iced::Task;
use log::{error, info};

use super::{Message, Spinner, Themes};
use crate::{services::hyprland_notify::report, utils::hyde_shell};

/// What a press on a theme chip leads to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SwitchDecision {
    /// A switch is already running for the named theme, so this press is
    /// dropped.
    AlreadySwitching(String),
    /// No theme of that name is installed, so nothing is asked of the desktop.
    NotInstalled,
    /// The desktop is asked to switch.
    Start
}

/// Decides what to do with a press on the chip of `theme`.
///
/// Kept apart from the module state so both refusals can be stated once and
/// checked without a `HyDE` install. They are refusals rather than best effort
/// for the same reason: a `HyDE` switch rewrites the whole desktop over several
/// seconds and is not reentrant, so a second one started on top of the first
/// races it over the state file, the wallpaper cache and every generated
/// stylesheet; and `HyDE`'s own switcher answers a name it does not know by
/// quietly keeping the current theme, which from the bar looks exactly like a
/// press that did nothing.
fn decide_switch(theme: &str, switching: Option<&str>, installed: &[String]) -> SwitchDecision {
    if let Some(pending) = switching {
        return SwitchDecision::AlreadySwitching(pending.to_owned());
    }

    if !installed.iter().any(|candidate| candidate == theme) {
        return SwitchDecision::NotInstalled;
    }

    SwitchDecision::Start
}

impl Themes {
    /// Hands a theme switch to the desktop, once it is worth handing over.
    ///
    /// Three things are settled before the desktop is disturbed, because each
    /// of them used to end in a module that claimed a switch nobody performed:
    /// a switch already under way is left alone, a theme that is not installed
    /// is refused outright — `HyDE`'s own switcher would silently keep the
    /// current one — and a missing switch script is reported instead of being
    /// logged where nobody looks.
    pub(super) fn switch(&mut self, theme: String, config: &Config) -> Task<Message> {
        if self.removing.is_some() {
            report(config, "a removal is running, switches must wait");
            return Task::none();
        }

        self.refresh();

        info!(
            "deciding `{theme}`: switching={:?} pending={:?}",
            self.switching, self.pending
        );

        match decide_switch(&theme, self.switching.as_deref(), &self.hyde.themes) {
            SwitchDecision::AlreadySwitching(running) => {
                if self.switching.as_deref() == Some(theme.as_str()) {
                    report(config, &format!("`{theme}` is being applied right now"));
                } else if self.pending.as_deref() == Some(theme.as_str()) {
                    report(config, &format!("`{theme}` is already queued next"));
                } else {
                    report(
                        config,
                        &format!("`{running}` is still being applied; `{theme}` is queued next")
                    );
                    self.pending = Some(theme);
                }

                return Task::none();
            }
            SwitchDecision::NotInstalled => {
                report(
                    config,
                    &format!("no HyDE theme named `{theme}` is installed")
                );
                return Task::none();
            }
            SwitchDecision::Start => {}
        }

        let command = match hyde_shell::switch_theme(&theme) {
            Ok(command) => command,
            Err(error) => {
                report(config, &format!("cannot switch the HyDE theme: {error}"));
                return Task::none();
            }
        };

        info!("switching the desktop to the HyDE theme `{theme}`");
        self.begin(theme.clone());

        Task::perform(hyde_shell::run(command), move |failure| Message::Switched {
            theme,
            failure
        })
    }

    /// Starts the wait on the switch to `theme`.
    ///
    /// The indicator is put back to its first frame here rather than left where
    /// the last switch abandoned it, so every wait looks the same from the
    /// press onwards instead of starting at whatever frame the previous one
    /// happened to end on.
    fn begin(&mut self, theme: String) {
        self.switching = Some(theme);
        self.spinner = Spinner::default();
    }

    /// Records what the desktop made of the switch that just ended.
    pub(super) fn switched(&mut self, theme: &str, failure: Option<&str>, config: &Config) {
        self.switching = None;
        self.refresh();

        match failure {
            Some(reason) => {
                error!("the switch to the HyDE theme `{theme}` failed: {reason}");
                report(
                    config,
                    &format!("the desktop refused to switch to `{theme}`")
                );
            }
            None => info!("the desktop finished switching to the HyDE theme `{theme}`")
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn installed() -> Vec<String> {
        vec!["Gruvbox Retro".to_owned(), "Tokyo Night".to_owned()]
    }

    #[test]
    fn an_installed_theme_is_handed_to_the_desktop() {
        assert_eq!(
            decide_switch("Tokyo Night", None, &installed()),
            SwitchDecision::Start
        );
    }

    #[test]
    fn a_theme_that_is_not_installed_is_refused_rather_than_attempted() {
        assert_eq!(
            decide_switch("Nordic Blue", None, &installed()),
            SwitchDecision::NotInstalled
        );
    }

    #[test]
    fn a_second_press_while_a_switch_runs_is_dropped() {
        assert_eq!(
            decide_switch("Tokyo Night", Some("Gruvbox Retro"), &installed()),
            SwitchDecision::AlreadySwitching("Gruvbox Retro".to_owned())
        );
    }

    #[test]
    fn pressing_the_theme_already_being_switched_to_does_not_start_it_twice() {
        assert_eq!(
            decide_switch("Tokyo Night", Some("Tokyo Night"), &installed()),
            SwitchDecision::AlreadySwitching("Tokyo Night".to_owned())
        );
    }

    #[test]
    fn a_machine_without_hyde_offers_no_theme_to_switch_to() {
        assert_eq!(
            decide_switch("Tokyo Night", None, &[]),
            SwitchDecision::NotInstalled
        );
    }

    #[test]
    fn the_indicator_returns_to_its_first_frame_for_every_new_switch() {
        let mut themes = Themes {
            switching: Some("Tokyo Night".to_owned()),
            ..Themes::default()
        };

        let _ = themes.update(Message::Tick, &Config::default());
        assert_ne!(themes.spinner(), Spinner::default());

        themes.begin("Gruvbox Retro".to_owned());

        assert_eq!(themes.switching(), Some("Gruvbox Retro"));
        assert_eq!(themes.spinner(), Spinner::default());
    }
}
